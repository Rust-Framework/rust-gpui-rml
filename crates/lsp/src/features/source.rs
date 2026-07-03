//! 补全数据源：统一封装 engine tags + props_registry + ProjectIndex
//!
//! 所有补全项的单一出口，避免各 handler 重复查询。

use lsp_types::Url;
use rust_rml_engine::compiler::props_registry;
use rust_rml_engine::tags;

use crate::workspace::project_index::ProjectIndex;

/// 补全项种类（供 LSP CompletionItemKind 映射）
#[derive(Debug, Clone)]
pub enum CompletionKind {
    /// HTML 标签 / 根标签 / 组件标签
    Tag { name: String, detail: String },
    /// 属性（static/bind/event）
    Prop { name: String, kind: PropKind, detail: String },
    /// 绑定路径（字段/computed 方法）
    BindingPath { name: String, detail: String },
    /// 命令方法名
    Command { name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Static,
    Bind,
    Event,
}

/// 属性集合
pub struct PropSet {
    pub statics: Vec<String>,
    pub binds: Vec<String>,
    pub events: Vec<String>,
}

/// 补全数据源
pub struct CompletionSource<'a> {
    index: &'a ProjectIndex,
}

impl<'a> CompletionSource<'a> {
    pub fn new(index: &'a ProjectIndex) -> Self {
        Self { index }
    }

    /// 所有可用标签（builtin HTML + root + 扩展组件 + item builder）
    pub fn tags(&self) -> Vec<CompletionKind> {
        let mut result = Vec::new();

        // 内置 HTML 标签
        const BUILTIN_TAGS: &[&str] = &[
            "div", "span", "p", "h1", "h2", "h3", "h4", "h5", "h6",
            "button", "input", "textarea", "ul", "ol", "li", "img", "a", "label", "br",
        ];
        for tag in BUILTIN_TAGS {
            result.push(CompletionKind::Tag {
                name: tag.to_string(),
                detail: "HTML element".to_string(),
            });
        }

        // 根标签
        const ROOT_TAGS: &[&str] = &["window", "modern_window", "tab_window", "dialog", "component"];
        for tag in ROOT_TAGS {
            result.push(CompletionKind::Tag {
                name: tag.to_string(),
                detail: "RML root element".to_string(),
            });
        }

        // 扩展组件（PascalCase）
        const EXTENSION_TAGS: &[&str] = &[
            "Button", "ButtonGroup", "Badge", "Checkbox", "Label", "Separator", "Tag",
            "Progress", "ProgressCircle", "Slider", "Switch", "Input", "TextInput",
            "TitleBar", "NativeStatusBar", "StatusBar", "ActivityBar", "Tree",
            "MenuBar", "menu", "status_bar", "Accordion", "AccordionItem",
        ];
        for tag in EXTENSION_TAGS {
            let detail = if tags::component_lookup(tag).is_some() {
                "gpui-component"
            } else {
                "component"
            };
            result.push(CompletionKind::Tag {
                name: tag.to_string(),
                detail: detail.to_string(),
            });
        }

        result
    }

    /// 查询标签的已注册属性（委托 props_registry）
    pub fn props_for(&self, tag: &str) -> PropSet {
        // 先查 shell 属性（window/modern_window/tab_window/dialog/component）
        if let Some(shell_props) = props_registry::shell_props_for(tag) {
            return PropSet {
                statics: shell_props.iter().map(|s| s.to_string()).collect(),
                binds: Vec::new(),
                events: Vec::new(),
            };
        }

        // 普通组件属性
        let (statics, binds, events) = props_registry::props_for(tag);
        PropSet {
            statics: statics.iter().map(|s| s.to_string()).collect(),
            binds: binds.iter().map(|s| s.to_string()).collect(),
            events: events.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// 绑定路径（observable_fields + computed_methods）
    pub fn binding_paths(&self, rml_uri: &Url) -> Vec<CompletionKind> {
        let Some(metadata_map) = self.index.metadata_for(rml_uri) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for meta in metadata_map.values() {
            for field in &meta.observable_fields {
                let ty = meta.field_types.get(field).cloned().unwrap_or_default();
                result.push(CompletionKind::BindingPath {
                    name: field.clone(),
                    detail: format!("observable field: {}", ty),
                });
            }
            for method in &meta.computed_methods {
                let ret = meta.computed_returns.get(method).cloned().unwrap_or_default();
                result.push(CompletionKind::BindingPath {
                    name: method.clone(),
                    detail: format!("computed method: () -> {}", ret),
                });
            }
        }
        result
    }

    /// 命令方法名
    pub fn commands(&self, rml_uri: &Url) -> Vec<CompletionKind> {
        let Some(metadata_map) = self.index.metadata_for(rml_uri) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for meta in metadata_map.values() {
            for cmd in &meta.commands {
                result.push(CompletionKind::Command { name: cmd.clone() });
            }
        }
        result
    }
}

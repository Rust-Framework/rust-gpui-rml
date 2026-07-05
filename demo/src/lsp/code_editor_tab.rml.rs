//! CodeEditorTab: 基于 InputState code_editor 模式的代码编辑器 Tab。
//!
//! 非贡献 Entity，由 MainWindow 直接管理。集成 LSP providers 提供补全/hover/跳转。
//! RML 声明式渲染：`<CodeEditor />` 自动应用 mono 字体 + size_full。
//!
//! `#[command]` 方法提供 format/rename/references/documentSymbol 入口；
//! format/rename 通过 `cx.active_window()` + `AnyWindowHandle::update` 获取
//! `&mut Window`，调用 `InputState::apply_lsp_edits` 将 LSP 编辑应用到编辑器。
//! references/documentSymbol 解析响应并格式化显示在 `last_lsp_result`。

use std::path::Path;
use std::sync::Arc;

use gpui_component::input::{InputState, TabSize};
use lsp_types::{DocumentSymbolResponse, Location, Position, TextEdit, Uri, WorkspaceEdit};
use rml::prelude::*;

use crate::lsp::{
    file_path_to_uri, LspClient, RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider,
};

#[component]
#[derive(Default)]
pub struct CodeEditorTab {
    editor_state: Option<Entity<InputState>>,
    lsp_client: Option<Arc<LspClient>>,
    uri: Option<Uri>,
    /// 最近一次 LSP 命令结果（供 UI 展示，每行一个元素）
    last_lsp_result: Vec<String>,
}

impl CodeEditorTab {
    pub fn new(
        file_path: &str,
        full_path: &Path,
        lsp_client: Arc<LspClient>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let text = std::fs::read_to_string(full_path).unwrap_or_default();
        let uri = file_path_to_uri(full_path).unwrap_or_else(|e| {
            log::error!("failed to create URI for {}: {e}", full_path.display());
            "file:///".parse::<Uri>().unwrap()
        });
        let language = if file_path.ends_with(".rml.rs") || file_path.ends_with(".rs") {
            "rust"
        } else {
            "rml"
        };

        lsp_client.open_document(&uri, &text, language);

        let editor_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language)
                .multi_line(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(&text);
            state.lsp.completion_provider =
                Some(std::rc::Rc::new(RmlCompletionProvider::new(
                    lsp_client.clone(),
                    uri.clone(),
                )));
            state.lsp.hover_provider =
                Some(std::rc::Rc::new(RmlHoverProvider::new(lsp_client.clone(), uri.clone())));
            state.lsp.definition_provider = Some(std::rc::Rc::new(RmlDefinitionProvider::new(
                lsp_client.clone(),
                uri.clone(),
            )));
            state
        });

        cx.new(|cx| {
            let uri_clone = uri.clone();
            let client_clone = lsp_client.clone();
            cx.observe(&editor_state, move |_, state, obs_cx| {
                let text = state.read(obs_cx).text().to_string();
                client_clone.change_document(&uri_clone, &text);
            })
            .detach();

            Self {
                editor_state: Some(editor_state),
                lsp_client: Some(lsp_client),
                uri: Some(uri),
                ..Default::default()
            }
        })
    }

    /// 当前编辑器光标位置（行/列从 0 开始）
    fn current_position(&self, cx: &Context<Self>) -> Option<Position> {
        let state = self.editor_state.as_ref()?.read(cx);
        let text = state.text().to_string();
        let cursor = state.cursor();
        let mut line = 0u32;
        let mut character = 0u32;
        for (i, ch) in text.char_indices() {
            if i >= cursor {
                break;
            }
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }
        Some(Position { line, character })
    }

    /// 格式化文档：调 LSP formatting，通过 apply_lsp_edits 应用到编辑器
    #[command]
    pub fn on_format_document(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.lsp_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let rx = client.formatting(&uri);
        cx.spawn(async move |this, cx| {
            match rx.recv() {
                Ok(Ok(value)) => {
                    let edits = parse_text_edits(&value);
                    let count = edits.len();
                    let _ = this.update(cx, |this, cx| {
                        if count > 0 {
                            apply_edits_to_editor(this, &edits, cx);
                        }
                        this.last_lsp_result = vec![format!("formatting: applied {count} edit(s)")];
                        cx.notify();
                    });
                    log::info!("LSP formatting: applied {count} edits for {}", uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP formatting error: {e}"),
                Err(e) => log::warn!("LSP formatting channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 重命名符号：在当前光标位置发起 rename，通过 apply_lsp_edits 应用到编辑器
    ///
    /// MVP：new_name 取 "renamed"（实际应弹输入框，待 UI 组件就绪后补齐）。
    #[command]
    pub fn on_rename_symbol(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.lsp_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let position = match self.current_position(cx) {
            Some(p) => p,
            None => return,
        };
        let rx = client.rename(&uri, position, "renamed");
        cx.spawn(async move |this, cx| {
            match rx.recv() {
                Ok(Ok(value)) => {
                    let edits = extract_edits_from_workspace_edit(&value, &uri);
                    let count = edits.len();
                    let _ = this.update(cx, |this, cx| {
                        if count > 0 {
                            apply_edits_to_editor(this, &edits, cx);
                        }
                        this.last_lsp_result = vec![format!("rename: applied {count} edit(s)")];
                        cx.notify();
                    });
                    log::info!("LSP rename: applied {count} edits for {}", uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP rename error: {e}"),
                Err(e) => log::warn!("LSP rename channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 查找引用：格式化显示所有引用位置（file:line:col）
    #[command]
    pub fn on_find_references(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.lsp_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let position = match self.current_position(cx) {
            Some(p) => p,
            None => return,
        };
        let rx = client.references(&uri, position, true);
        cx.spawn(async move |this, cx| {
            match rx.recv() {
                Ok(Ok(value)) => {
                    let locations = parse_locations(&value);
                    let lines = format_references(&locations);
                    let _ = this.update(cx, |this, cx| {
                        this.last_lsp_result = lines;
                        cx.notify();
                    });
                    log::info!("LSP references: {} locations for {}", locations.len(), uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP references error: {e}"),
                Err(e) => log::warn!("LSP references channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 显示文档符号：格式化显示符号名称列表
    #[command]
    pub fn on_show_document_symbols(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.lsp_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let rx = client.document_symbol(&uri);
        cx.spawn(async move |this, cx| {
            match rx.recv() {
                Ok(Ok(value)) => {
                    let lines = format_document_symbols(&value);
                    let _ = this.update(cx, |this, cx| {
                        this.last_lsp_result = lines;
                        cx.notify();
                    });
                    log::info!("LSP documentSymbol for {}", uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP documentSymbol error: {e}"),
                Err(e) => log::warn!("LSP documentSymbol channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }
}

/// 将 LSP formatting 响应解析为 TextEdit 列表
fn parse_text_edits(value: &serde_json::Value) -> Vec<TextEdit> {
    if value.is_null() {
        Vec::new()
    } else {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }
}

/// 从 WorkspaceEdit 中提取指定 URI 的 TextEdit 列表
fn extract_edits_from_workspace_edit(value: &serde_json::Value, uri: &Uri) -> Vec<TextEdit> {
    let workspace_edit: WorkspaceEdit = match serde_json::from_value(value.clone()) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    workspace_edit
        .changes
        .and_then(|c| c.get(uri).cloned())
        .unwrap_or_default()
}

/// 通过 active_window + apply_lsp_edits 将编辑应用到编辑器
fn apply_edits_to_editor(
    this: &mut CodeEditorTab,
    edits: &[TextEdit],
    cx: &mut Context<CodeEditorTab>,
) {
    let Some(editor_state) = this.editor_state.as_ref() else {
        return;
    };
    let Some(handle) = cx.active_window() else {
        log::warn!("apply_edits: no active window");
        return;
    };
    let edits_vec = edits.to_vec();
    let _ = handle.update(&mut **cx, |_view, window, app_cx| {
        editor_state.update(app_cx, |state, state_cx| {
            state.apply_lsp_edits(&edits_vec, window, state_cx);
        });
    });
}

/// 将 LSP references 响应解析为 Location 列表
fn parse_locations(value: &serde_json::Value) -> Vec<Location> {
    if value.is_null() {
        Vec::new()
    } else {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }
}

/// 格式化引用列表为行向量（file:line:col，最多显示 5 条）
fn format_references(locations: &[Location]) -> Vec<String> {
    if locations.is_empty() {
        return vec!["references: 0 location(s)".to_string()];
    }
    let count = locations.len();
    let mut lines = vec![format!("references: {count} location(s)")];
    for loc in locations.iter().take(5) {
        let path = uri_to_short_path(&loc.uri);
        let line = loc.range.start.line + 1;
        let col = loc.range.start.character + 1;
        lines.push(format!("  {path}:{line}:{col}"));
    }
    if count > 5 {
        lines.push(format!("  ... +{} more", count - 5));
    }
    lines
}

/// 格式化文档符号为行向量（名称列表，最多显示 10 条）
fn format_document_symbols(value: &serde_json::Value) -> Vec<String> {
    let response: DocumentSymbolResponse = serde_json::from_value(value.clone())
        .unwrap_or(DocumentSymbolResponse::Flat(Vec::new()));
    let names: Vec<String> = match response {
        DocumentSymbolResponse::Flat(symbols) => {
            symbols.iter().take(10).map(|s| s.name.to_string()).collect()
        }
        DocumentSymbolResponse::Nested(symbols) => {
            symbols.iter().take(10).map(|s| s.name.to_string()).collect()
        }
    };
    if names.is_empty() {
        return vec!["documentSymbol: 0 symbol(s)".to_string()];
    }
    let count = names.len();
    let mut lines = vec![format!("documentSymbol: {count} symbol(s)")];
    lines.push(names.join(", "));
    lines
}

/// 将 file:// URI 转换为简短路径（仅文件名）
fn uri_to_short_path(uri: &Uri) -> String {
    let s = uri.as_str();
    if let Some(idx) = s.rfind('/') {
        s[idx + 1..].to_string()
    } else {
        s.to_string()
    }
}

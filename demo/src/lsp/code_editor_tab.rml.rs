//! CodeEditorTab: 基于 InputState code_editor 模式的代码编辑器 Tab。
//!
//! 非贡献 Entity，由 MainWindow 直接管理。集成 LSP providers 提供补全/hover/跳转。
//! RML 声明式渲染：`<CodeEditor />` 自动应用 mono 字体 + size_full。
//!
//! `#[command]` 方法提供 format/rename/references/documentSymbol 入口；
//! InputState 的 set_value/insert/replace 均需 `&mut Window`，而 #[command]
//! 方法签名约定不含 window，故命令仅发起 LSP 请求并记录结果到 `last_lsp_result`
//! 字段供 UI 展示。文本应用待后续通过 on_loaded 注入的窗口句柄或专用 action 实现。

use std::path::Path;
use std::sync::Arc;

use gpui_component::input::{InputState, TabSize};
use lsp_types::{Position, Uri};
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
    /// 最近一次 LSP 命令结果（供 UI 展示）
    last_lsp_result: String,
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

    /// 格式化文档：调 LSP formatting，结果记录到 last_lsp_result
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
                    let summary = format!("formatting: {} bytes", value.to_string().len());
                    let _ = this.update(cx, |this, cx| {
                        this.last_lsp_result = summary;
                        cx.notify();
                    });
                    log::info!("LSP formatting ok for {}", uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP formatting error: {e}"),
                Err(e) => log::warn!("LSP formatting channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 重命名符号：在当前光标位置发起 rename，结果记录到 last_lsp_result
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
                    let summary = format!("rename: {} changes", count_edits(&value));
                    let _ = this.update(cx, |this, cx| {
                        this.last_lsp_result = summary;
                        cx.notify();
                    });
                    log::info!("LSP rename ok for {}", uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP rename error: {e}"),
                Err(e) => log::warn!("LSP rename channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 查找引用：在日志与 last_lsp_result 中输出引用计数
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
                    let count = value.as_array().map(|a| a.len()).unwrap_or(0);
                    let summary = format!("references: {count} location(s)");
                    let _ = this.update(cx, |this, cx| {
                        this.last_lsp_result = summary;
                        cx.notify();
                    });
                    log::info!("LSP references: {count} for {}", uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP references error: {e}"),
                Err(e) => log::warn!("LSP references channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 显示文档符号：在日志与 last_lsp_result 中输出符号计数
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
                    let count = value.as_array().map(|a| a.len()).unwrap_or(0);
                    let summary = format!("documentSymbol: {count} symbol(s)");
                    let _ = this.update(cx, |this, cx| {
                        this.last_lsp_result = summary;
                        cx.notify();
                    });
                    log::info!("LSP documentSymbol: {count} for {}", uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP documentSymbol error: {e}"),
                Err(e) => log::warn!("LSP documentSymbol channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }
}

/// 统计 WorkspaceEdit 中 TextEdit 总数
fn count_edits(value: &serde_json::Value) -> usize {
    value
        .get("changes")
        .and_then(|c| c.as_object())
        .map(|m| m.values().filter_map(|v| v.as_array()).map(|a| a.len()).sum())
        .unwrap_or(0)
}

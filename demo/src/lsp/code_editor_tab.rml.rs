//! CodeEditorTab: 基于 InputState code_editor 模式的代码编辑器 Tab。
//!
//! 非贡献 Entity，由 MainWindow 直接管理。集成 LSP providers 提供补全/hover/跳转。
//! RML 声明式渲染：`<CodeEditor />` 自动应用 mono 字体 + size_full。
//!
//! `#[command]` 方法提供 format/rename/references/documentSymbol 入口；
//! format/rename 通过 `cx.active_window()` + `AnyWindowHandle::update` 获取
//! `&mut Window`，调用 `InputState::apply_lsp_edits` 将 LSP 编辑应用到编辑器。
//! references/documentSymbol 解析响应并将摘要写入 `LspStatusState`（状态栏显示）。

use std::path::Path;
use std::sync::Arc;

use gpui_component::input::{InputState, TabSize};
use lsp_types::{DocumentSymbolResponse, Location, Position, TextEdit, Uri, WorkspaceEdit};
use rml::prelude::*;

use crate::lsp::{
    file_path_to_uri, LspClient, LspStatusStateRef, RmlCompletionProvider, RmlDefinitionProvider,
    RmlHoverProvider,
};

#[component]
#[derive(Default)]
pub struct CodeEditorTab {
    editor_state: Option<Entity<InputState>>,
    lsp_client: Option<Arc<LspClient>>,
    uri: Option<Uri>,
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
                        set_lsp_status(cx, format!("formatting: applied {count} edit(s)"));
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
                        set_lsp_status(cx, format!("rename: applied {count} edit(s)"));
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

    /// 查找引用：将引用计数摘要写入状态栏
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
                    let count = locations.len();
                    let _ = this.update(cx, |_, cx| {
                        set_lsp_status(cx, format!("references: {count} location(s)"));
                    });
                    log::info!("LSP references: {} locations for {}", count, uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP references error: {e}"),
                Err(e) => log::warn!("LSP references channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 显示文档符号：将符号计数摘要写入状态栏
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
                    let count = count_document_symbols(&value);
                    let _ = this.update(cx, |_, cx| {
                        set_lsp_status(cx, format!("documentSymbol: {count} symbol(s)"));
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

/// 将摘要消息写入 `LspStatusState`（经 IAppContext 服务查询）。
fn set_lsp_status(cx: &mut Context<CodeEditorTab>, message: String) {
    if let Some(entity) = cx
        .get_service::<LspStatusStateRef>()
        .and_then(|r| r.0.upgrade())
    {
        entity.update(cx, |state, state_cx| state.set_message(message, state_cx));
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

/// 统计文档符号数量（documentSymbol 响应）
fn count_document_symbols(value: &serde_json::Value) -> usize {
    let response: DocumentSymbolResponse = serde_json::from_value(value.clone())
        .unwrap_or(DocumentSymbolResponse::Flat(Vec::new()));
    match response {
        DocumentSymbolResponse::Flat(symbols) => symbols.len(),
        DocumentSymbolResponse::Nested(symbols) => symbols.len(),
    }
}

//! CodeEditorTab: 基于 InputState code_editor 模式的代码编辑器 Tab。
//!
//! 非贡献 Entity，由 MainWindow 直接管理。集成 LSP providers 提供补全/hover/跳转。
//! RML 声明式渲染：`<CodeEditor />` 自动应用 mono 字体 + size_full。
//!
//! `#[command]` 方法提供 format/rename/references/documentSymbol 入口；
//! format/rename 通过 `cx.active_window()` + `AnyWindowHandle::update` 获取
//! `&mut Window`，调用 `InputState::apply_lsp_edits` 将 LSP 编辑应用到编辑器。
//! references/documentSymbol 解析响应并将摘要写入 `LspStatusState`（状态栏显示）。
//!
//! 右键菜单通过 NativeMenu + Action 派发，覆盖 format/rename/goto/refs/symbols。
//! 面包屑导航对接 documentSymbol，随光标移动更新。

use std::path::Path;
use std::sync::Arc;

use gpui::{Action, Window};
use gpui_component::input::{InputState, TabSize};
use gpui_component::native_menu::NativeMenu;
use lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, Location, Position, Range, TextEdit, Uri, WorkspaceEdit,
};
use rml::prelude::*;
use rml_ui::BreadcrumbItem;
use rust_rml_client::{file_path_to_uri, LanguageClient};
use serde::Deserialize;

use crate::lsp::LspStatusStateRef;

/// 格式化文档 Action（右键菜单派发）
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = code_editor, no_json)]
struct FormatDocument;

/// 重命名符号 Action
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = code_editor, no_json)]
struct RenameSymbol;

/// 查找引用 Action
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = code_editor, no_json)]
struct FindReferences;

/// 跳转定义 Action
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = code_editor, no_json)]
struct GoToDefinition;

/// 显示文档符号 Action
#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = code_editor, no_json)]
struct ShowDocumentSymbols;

#[component]
#[derive(Default)]
pub struct CodeEditorTab {
    editor_state: Option<Entity<InputState>>,
    language_client: Option<Arc<LanguageClient>>,
    uri: Option<Uri>,
    /// 文档符号列表（documentSymbol 响应），用于面包屑路径计算
    document_symbols: Vec<DocumentSymbol>,
    /// 面包屑导航项（绑定到 `<Breadcrumb items={breadcrumb_items} />`）
    breadcrumb_items: Vec<BreadcrumbItem>,
}

impl CodeEditorTab {
    pub fn new(
        file_path: &str,
        full_path: &Path,
        language_client: Arc<LanguageClient>,
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

        language_client.open_document(&uri, &text);

        let editor_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language)
                .multi_line(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(&text);
            language_client.install_providers(&mut state, uri.clone());
            state
        });

        cx.new(|cx| {
            let uri_clone = uri.clone();
            let client_clone = language_client.clone();
            cx.observe(&editor_state, move |_: &mut Self, state, obs_cx| {
                let text = state.read(obs_cx).text().to_string();
                client_clone.change_document(&uri_clone, &text);
            })
            .detach();

            // 订阅 editor_state 变化以更新面包屑（光标移动会触发 cx.notify）
            cx.observe(&editor_state, |this: &mut Self, _state, cx| {
                this.update_breadcrumb(cx);
            })
            .detach();

            let mut this = Self {
                editor_state: Some(editor_state),
                language_client: Some(language_client.clone()),
                uri: Some(uri),
                ..Default::default()
            };
            this.fetch_document_symbols(cx);
            this
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

    // ──────────────────────────────────────────────────────────────────────
    //  do_* 方法：业务逻辑实现（供 #[command] 和 action handler 共用）
    // ──────────────────────────────────────────────────────────────────────

    /// 格式化文档：调 LSP formatting，通过 apply_lsp_edits 应用到编辑器
    fn do_format_document(&mut self, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.language_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let rx = client.lsp().formatting(&uri);
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
    fn do_rename_symbol(&mut self, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.language_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let position = match self.current_position(cx) {
            Some(p) => p,
            None => return,
        };
        let rx = client.lsp().rename(&uri, position, "renamed");
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
    fn do_find_references(&mut self, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.language_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let position = match self.current_position(cx) {
            Some(p) => p,
            None => return,
        };
        let rx = client.lsp().references(&uri, position, true);
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

    /// 跳转定义：将目标位置摘要写入状态栏
    fn do_goto_definition(&mut self, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.language_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let position = match self.current_position(cx) {
            Some(p) => p,
            None => return,
        };
        let rx = client.lsp().definition(&uri, position);
        cx.spawn(async move |this, cx| {
            match rx.recv() {
                Ok(Ok(value)) => {
                    if let Some(loc) = parse_definition(&value) {
                        let uri_str = loc.uri.as_str().to_string();
                        let line = loc.range.start.line + 1;
                        let col = loc.range.start.character + 1;
                        let _ = this.update(cx, |_, cx| {
                            set_lsp_status(cx, format!("goto: {uri_str}:{line}:{col}"));
                        });
                        log::info!("LSP goto: {uri_str}:{line}:{col}");
                    } else {
                        let _ = this.update(cx, |_, cx| {
                            set_lsp_status(cx, "goto: no definition".to_string());
                        });
                    }
                }
                Ok(Err(e)) => log::warn!("LSP goto error: {e}"),
                Err(e) => log::warn!("LSP goto channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 显示文档符号：将符号计数摘要写入状态栏，并刷新面包屑数据源
    fn do_show_document_symbols(&mut self, cx: &mut Context<Self>) {
        self.fetch_document_symbols(cx);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  #[command] 方法：toolbar 按钮入口，委托给 do_*
    // ──────────────────────────────────────────────────────────────────────

    #[command]
    pub fn on_format_document(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_format_document(cx);
    }

    #[command]
    pub fn on_rename_symbol(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_rename_symbol(cx);
    }

    #[command]
    pub fn on_find_references(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_find_references(cx);
    }

    #[command]
    pub fn on_show_document_symbols(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_show_document_symbols(cx);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  Action handler 方法：右键菜单 Action 派发入口，委托给 do_*
    // ──────────────────────────────────────────────────────────────────────

    fn on_format_action(&mut self, _: &FormatDocument, _: &mut Window, cx: &mut Context<Self>) {
        self.do_format_document(cx);
    }

    fn on_rename_action(&mut self, _: &RenameSymbol, _: &mut Window, cx: &mut Context<Self>) {
        self.do_rename_symbol(cx);
    }

    fn on_find_references_action(
        &mut self,
        _: &FindReferences,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.do_find_references(cx);
    }

    fn on_goto_definition_action(
        &mut self,
        _: &GoToDefinition,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.do_goto_definition(cx);
    }

    fn on_show_document_symbols_action(
        &mut self,
        _: &ShowDocumentSymbols,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.do_show_document_symbols(cx);
    }

    // ──────────────────────────────────────────────────────────────────────
    //  右键菜单构建
    // ──────────────────────────────────────────────────────────────────────

    /// 构建 CodeEditor 右键菜单：format / rename / goto / references / symbols
    ///
    /// 由 RML `<CodeEditor context-menu="build_editor_menu" />` 调用，
    /// 闭包桥接：`__view.update(c, |this, cx| this.build_editor_menu(menu, w, cx))`。
    pub fn build_editor_menu(
        &mut self,
        menu: NativeMenu,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> NativeMenu {
        menu.menu("Format Document", Box::new(FormatDocument))
            .menu("Rename Symbol", Box::new(RenameSymbol))
            .separator()
            .menu("Go to Definition", Box::new(GoToDefinition))
            .menu("Find References", Box::new(FindReferences))
            .separator()
            .menu("Show Document Symbols", Box::new(ShowDocumentSymbols))
    }

    // ──────────────────────────────────────────────────────────────────────
    //  面包屑数据流
    // ──────────────────────────────────────────────────────────────────────

    /// 异步拉取 documentSymbol，刷新 document_symbols + breadcrumb_items
    fn fetch_document_symbols(&mut self, cx: &mut Context<Self>) {
        let (client, uri) = match (&self.language_client, &self.uri) {
            (Some(c), Some(u)) => (c.clone(), u.clone()),
            _ => return,
        };
        let rx = client.lsp().document_symbol(&uri);
        cx.spawn(async move |this, cx| {
            match rx.recv() {
                Ok(Ok(value)) => {
                    let symbols = parse_document_symbols(&value);
                    let count = symbols.len();
                    let _ = this.update(cx, |this, cx| {
                        this.document_symbols = symbols;
                        this.update_breadcrumb(cx);
                        set_lsp_status(cx, format!("documentSymbol: {count} symbol(s)"));
                    });
                    log::info!("LSP documentSymbol for {} ({count} symbols)", uri.as_str());
                }
                Ok(Err(e)) => log::warn!("LSP documentSymbol error: {e}"),
                Err(e) => log::warn!("LSP documentSymbol channel: {e}"),
            }
            Ok::<(), anyhow::Error>(())
        })
        .detach();
    }

    /// 根据当前光标位置和 document_symbols 计算面包屑路径
    fn update_breadcrumb(&mut self, cx: &mut Context<Self>) {
        let position = match self.current_position(cx) {
            Some(p) => p,
            None => {
                if !self.breadcrumb_items.is_empty() {
                    self.breadcrumb_items = Vec::new();
                    cx.notify();
                }
                return;
            }
        };
        let path = find_symbol_path(&self.document_symbols, &position);
        let new_items: Vec<BreadcrumbItem> = path
            .iter()
            .map(|s| BreadcrumbItem::new(s.name.clone()))
            .collect();
        if new_items.len() != self.breadcrumb_items.len()
            || new_items
                .iter()
                .zip(self.breadcrumb_items.iter())
                .any(|(n, o)| n.label != o.label)
        {
            self.breadcrumb_items = new_items;
            cx.notify();
        }
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

/// 将 LSP definition 响应解析为首个 Location（支持 Array/Single 两种格式）
fn parse_definition(value: &serde_json::Value) -> Option<Location> {
    if value.is_null() {
        return None;
    }
    if let Ok(locations) = serde_json::from_value::<Vec<Location>>(value.clone()) {
        return locations.into_iter().next();
    }
    serde_json::from_value::<Location>(value.clone()).ok()
}

/// 将 documentSymbol 响应解析为嵌套 DocumentSymbol 列表
///
/// 仅接受 Nested 格式（含 children）；Flat 格式不支持嵌套路径，跳过。
fn parse_document_symbols(value: &serde_json::Value) -> Vec<DocumentSymbol> {
    let response: DocumentSymbolResponse = serde_json::from_value(value.clone())
        .unwrap_or(DocumentSymbolResponse::Nested(Vec::new()));
    match response {
        DocumentSymbolResponse::Flat(_) => Vec::new(),
        DocumentSymbolResponse::Nested(symbols) => symbols,
    }
}

/// 从嵌套 DocumentSymbol 树中查找包含 position 的根到叶路径
fn find_symbol_path(symbols: &[DocumentSymbol], position: &Position) -> Vec<DocumentSymbol> {
    for sym in symbols {
        if range_contains(&sym.range, position) {
            let mut path = vec![sym.clone()];
            if let Some(children) = &sym.children {
                let deeper = find_symbol_path(children, position);
                if !deeper.is_empty() {
                    path.extend(deeper);
                }
            }
            return path;
        }
    }
    Vec::new()
}

/// 判断 position 是否在 range 范围内（含边界）
fn range_contains(range: &Range, pos: &Position) -> bool {
    let start_ok = pos.line > range.start.line
        || (pos.line == range.start.line && pos.character >= range.start.character);
    let end_ok = pos.line < range.end.line
        || (pos.line == range.end.line && pos.character <= range.end.character);
    start_ok && end_ok
}

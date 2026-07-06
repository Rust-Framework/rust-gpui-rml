//! LSP 状态栏贡献 —— 跨组件通信演示。
//!
//! `LspStatusState` Entity 持有最近一次 LSP 命令摘要，经 IAppContext 注册为单例。
//! `CodeEditorTab`（producer）经 `get_service::<LspStatusStateRef>()` 写入 →
//! `LspStatusItem`（consumer，status 贡献）在 `render` 中读取。
//! `MainWindow` observe Entity → `cx.notify` → `render_status_bar` 重渲。

use std::sync::Once;

use gpui::{AnyElement, App, Context, ParentElement, SharedString, Styled, WeakEntity, Window};
use rml::prelude::*;
use rml_core::contribution::register_visual_ability;
use rml_core::i18n::t_static;

/// LSP 状态栏状态 Entity —— 持有最近一次 LSP 命令的摘要消息。
pub struct LspStatusState {
    message: Option<String>,
}

impl LspStatusState {
    pub fn new() -> Self {
        Self { message: None }
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn set_message(&mut self, message: String, cx: &mut Context<Self>) {
        self.message = Some(message);
        cx.notify();
    }
}

impl Default for LspStatusState {
    fn default() -> Self {
        Self::new()
    }
}

/// `LspStatusState` 弱引用——经 IAppContext 注册为单例，
/// `CodeEditorTab` 经 `get_service::<LspStatusStateRef>()` 查询并更新。
pub struct LspStatusStateRef(pub WeakEntity<LspStatusState>);

/// 状态栏贡献：显示最近一次 LSP 命令结果摘要。
#[contribute(host_id = "demo.shell", id = "status.lsp", kind = "status", order = 10)]
#[derive(Default)]
pub struct LspStatusItem;

impl IContribution for LspStatusItem {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.status_lsp")
    }
}

impl IVisual for LspStatusItem {
    fn render(&self, _window: &mut Window, cx: &mut App) -> AnyElement {
        let msg = cx
            .get_service::<LspStatusStateRef>()
            .and_then(|r| r.0.upgrade())
            .and_then(|entity| entity.read(cx).message().map(|s| s.to_string()));
        match msg {
            Some(m) => gpui::div().text_xs().child(m).into_any_element(),
            None => gpui::div().into_any_element(),
        }
    }
}

static LSP_STATUS_ITEM_REGISTERED: Once = Once::new();

/// 注册 `LspStatusItem` 的 `IVisual` 能力 cast。
///
/// `LspStatusItem` 有 `#[contribute]` 无 `#[component]`，视觉能力不自动注册。
/// 需在 `MainWindow::on_loaded` 的 `bootstrap_host_contributions` 后调用。
pub fn ensure_lsp_status_item_registered() {
    LSP_STATUS_ITEM_REGISTERED.call_once(|| {
        register_visual_ability::<LspStatusItem>();
    });
}

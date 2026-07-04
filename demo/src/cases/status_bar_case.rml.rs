use std::sync::Once;

use gpui::{AnyElement, ParentElement, SharedString, Styled};
use rml::prelude::*;
use rml_core::contribution::{register_visual_ability, IVisualContribution};
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.status_bar",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct StatusBarCase {}

impl IContribution for StatusBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.status_bar.title")
    }
}

impl ILifecycle for StatusBarCase {}

impl StatusBarCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IContribution for StatusReady {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("shell.status_ready").into() }
}"#
            .to_string()
    }
}

/// 状态栏贡献：演示 status slot（从 shell_meta.rs 迁入）
#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IContribution for StatusReady {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.status_ready")
    }
}

impl IVisualContribution for StatusReady {
    fn render(&self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> AnyElement {
        gpui::div()
            .text_xs()
            .child(t_static("shell.status_ready"))
            .into_any_element()
    }
}

static STATUS_READY_REGISTERED: Once = Once::new();

/// 注册 `StatusReady` 的 `IVisualContribution` 能力 cast。
///
/// `StatusReady` 有 `#[contribute]` 无 `#[component]`，视觉能力不自动注册。
/// 需在 `MainWindow::on_loaded` 的 `project_entries()` 前调用，使 `as_visual()` 查询生效。
pub fn ensure_status_ready_registered() {
    STATUS_READY_REGISTERED.call_once(|| {
        register_visual_ability::<StatusReady>();
    });
}

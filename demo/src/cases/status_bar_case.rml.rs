use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
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
        t_static("case.status_bar.title").into()
    }
}

impl ILifecycle for StatusBarCase {}

/// 状态栏贡献：演示 status slot（从 shell_meta.rs 迁入）
#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IContribution for StatusReady {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.status_ready").into()
    }
}

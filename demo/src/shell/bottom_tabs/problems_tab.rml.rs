use gpui::SharedString;
use rml::prelude::*;
use rml_core::contribution::IContribution;
use rml_core::i18n::t_static;

/// Bottom 面板 Problems Tab 视觉贡献。
#[contribute(
    host_id = "demo.shell",
    id = "demo.bottom.problems",
    kind = "bottom_tab",
    order = 2
)]
#[component]
#[derive(Default)]
pub struct ProblemsTab {
    _priv: (),
}

impl IContribution for ProblemsTab {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }

    fn name(&self) -> SharedString {
        t_static("bottom_tab.problems")
    }
}

impl ILifecycle for ProblemsTab {}

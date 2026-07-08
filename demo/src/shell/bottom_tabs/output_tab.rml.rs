use gpui::SharedString;
use rml::prelude::*;
use rml_core::contribution::IContribution;
use rml_core::i18n::t_static;

/// Bottom 面板 Output Tab 视觉贡献。
#[contribute(
    host_id = "demo.shell",
    id = "demo.bottom.output",
    kind = "bottom_tab",
    order = 1
)]
#[component]
#[derive(Default)]
pub struct OutputTab {
    _priv: (),
}

impl IContribution for OutputTab {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }

    fn name(&self) -> SharedString {
        t_static("bottom_tab.output")
    }
}

impl ILifecycle for OutputTab {}

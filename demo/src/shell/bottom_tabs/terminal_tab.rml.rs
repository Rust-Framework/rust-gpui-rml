use gpui::SharedString;
use rml::prelude::*;
use rml_core::contribution::IContribution;
use rml_core::i18n::t_static;

/// Bottom 面板 Terminal Tab 视觉贡献。
#[contribute(
    host_id = "demo.shell",
    id = "demo.bottom.terminal",
    kind = "bottom_tab",
    order = 0
)]
#[component]
#[derive(Default)]
pub struct TerminalTab {
    _priv: (),
}

impl IContribution for TerminalTab {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }

    fn name(&self) -> SharedString {
        t_static("bottom_tab.terminal")
    }
}

impl ILifecycle for TerminalTab {}

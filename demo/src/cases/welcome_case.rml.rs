use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "welcome",
    kind = "case",
    order = 0,
)]
#[component]
#[derive(Default)]
pub struct WelcomeCase {}

impl IContribution for WelcomeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.welcome").into()
    }
}

impl ILifecycle for WelcomeCase {}

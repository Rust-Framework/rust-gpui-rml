use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.native_status_bar",
    kind = "case",
    group = "components",
    order = 32,
)]
#[component]
#[derive(Default)]
pub struct NativeStatusBarCase {}

impl IContribution for NativeStatusBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.native_status_bar.title")
    }
}

impl NativeStatusBarCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<NativeStatusBar>
    <span>就绪</span>
</NativeStatusBar>"#
            .to_string()
    }
}

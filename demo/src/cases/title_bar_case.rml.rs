use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.title_bar",
    kind = "case",
    group = "components",
    order = 31,
)]
#[component]
#[derive(Default)]
pub struct TitleBarCase {}

impl IContribution for TitleBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.title_bar.title")
    }
}

impl TitleBarCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<TitleBar>
    <Button label="菜单" ghost="" />
</TitleBar>"#
            .to_string()
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.button_group",
    kind = "case",
    group = "components",
    order = 28,
)]
#[component]
#[derive(Default)]
pub struct ButtonGroupCase {}

impl IContribution for ButtonGroupCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.button_group.title")
    }
}

impl ButtonGroupCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<ButtonGroup>
    <Button label="上一步" />
    <Button label="下一步" />
</ButtonGroup>"#
            .to_string()
    }
}

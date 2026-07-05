use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.label",
    kind = "case",
    group = "components",
    order = 23,
)]
#[component]
#[derive(Default)]
pub struct LabelCase {}

impl IContribution for LabelCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.label.title")
    }
}

impl LabelCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Label label="用户名" />
<Label>用户名</Label>
<Label label={dynamic_title} />"#
            .to_string()
    }
}

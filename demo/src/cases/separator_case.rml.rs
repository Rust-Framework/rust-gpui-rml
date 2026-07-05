use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.separator",
    kind = "case",
    group = "components",
    order = 24,
)]
#[component]
#[derive(Default)]
pub struct SeparatorCase {}

impl IContribution for SeparatorCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.separator.title")
    }
}

impl SeparatorCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Separator />
<Separator vertical="" />
<Separator dashed="" />"#
            .to_string()
    }
}

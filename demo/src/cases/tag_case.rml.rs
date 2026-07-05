use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.tag",
    kind = "case",
    group = "components",
    order = 25,
)]
#[component]
#[derive(Default)]
pub struct TagCase {}

impl IContribution for TagCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tag.title")
    }
}

impl TagCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Tag>Default</Tag>
<Tag primary="">Primary</Tag>
<Tag danger="">Danger</Tag>
<Tag size="small">Small</Tag>"#
            .to_string()
    }
}

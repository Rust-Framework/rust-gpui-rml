use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.badge",
    kind = "case",
    group = "components",
    order = 22,
)]
#[component]
#[derive(Default)]
pub struct BadgeCase {}

impl IContribution for BadgeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.badge.title")
    }
}

impl BadgeCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Badge>5</Badge>
<Badge dot="">New</Badge>
<Badge count={99}>Messages</Badge>"#
            .to_string()
    }
}

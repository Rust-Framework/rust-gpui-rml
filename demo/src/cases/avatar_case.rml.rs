use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.avatar",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct AvatarCase {}

impl IContribution for AvatarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.avatar.title")
    }
}

impl AvatarCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Avatar src="https://..." size="large" />
<Avatar name="Jason Lee" />
<Avatar placeholder="Building2" />
<AvatarGroup limit="3" ellipsis="">
    <Avatar src="..." />
    <Avatar name="John" />
</AvatarGroup>"#
            .to_string()
    }
}

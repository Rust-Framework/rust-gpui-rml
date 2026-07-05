use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.avatar_group",
    kind = "case",
    group = "components",
    order = 29,
)]
#[component]
#[derive(Default)]
pub struct AvatarGroupCase {}

impl IContribution for AvatarGroupCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.avatar_group.title")
    }
}

impl AvatarGroupCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<AvatarGroup limit="3" ellipsis="">
    <Avatar name="Alice" />
    <Avatar name="Bob" />
    <Avatar name="Charlie" />
    <Avatar name="Dave" />
</AvatarGroup>"#
            .to_string()
    }
}

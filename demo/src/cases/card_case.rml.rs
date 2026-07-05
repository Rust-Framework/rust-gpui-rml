use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.card",
    kind = "case",
    group = "components",
    order = 30,
)]
#[component]
#[derive(Default)]
pub struct CardCase {}

impl IContribution for CardCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.card.title")
    }
}

impl CardCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Card title="标题">
    <p>卡片内容</p>
</Card>

<Card title="标题" hoverable="">
    <p>可悬浮卡片</p>
</Card>"#
            .to_string()
    }
}

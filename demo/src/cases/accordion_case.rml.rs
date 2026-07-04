use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.accordion",
    kind = "case",
    group = "components",
    order = 10,
)]
#[component]
#[derive(Default)]
pub struct AccordionCase {
    pub last_open: String,
}

impl IContribution for AccordionCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.accordion.title")
    }
}

impl AccordionCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.last_open.is_empty() {
            "尚未切换任何项".to_string()
        } else {
            format!("上次展开项索引：{}", self.last_open)
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<accordion bordered="">
    <item title="第一项" open="">
        <p>内容</p>
    </item>
    <item title="第二项">
        <p>内容</p>
    </item>
</accordion>"#
            .to_string()
    }

    #[command]
    pub fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>) {
        self.last_open = format!("{:?}", open_ixs);
        cx.notify();
    }
}

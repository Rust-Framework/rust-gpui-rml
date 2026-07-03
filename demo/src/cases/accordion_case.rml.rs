use rml::prelude::*;

#[contribute(
    host_id = "demo.activity",
    id = "components.accordion",
    name = "case.accordion.title",
    kind = "case",
    group = "components",
    order = 13,
)]
#[component]
#[derive(Default)]
pub struct AccordionCase {
    pub last_open: String,
}

impl ILifecycle for AccordionCase {}

impl AccordionCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.last_open.is_empty() {
            "尚未切换任何项".to_string()
        } else {
            format!("上次展开项索引：{}", self.last_open)
        }
    }

    #[command]
    pub fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>) {
        self.last_open = format!("{:?}", open_ixs);
        cx.notify();
    }
}

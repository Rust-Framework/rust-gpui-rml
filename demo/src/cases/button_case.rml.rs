use rml::prelude::*;

#[contribute(
    host_id = "demo.activity",
    id = "components.button",
    name = "case.button.title",
    kind = "case",
    group = "components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub button_clicks: i32,
}

impl ILifecycle for ButtonCase {}

impl ButtonCase {
    #[computed]
    pub fn button_demo_text(&self) -> String {
        format!("按钮点击：{}", self.button_clicks)
    }

    #[command]
    pub fn on_button_demo_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.button_clicks += 1;
    }
}

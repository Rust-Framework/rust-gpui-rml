use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "binding.counter",
    kind = "case",
    group = "binding",
    order = 1,
)]
#[component]
#[derive(Default)]
pub struct CounterCase {
    pub count: i32,
}

impl IContribution for CounterCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.counter.title").into()
    }
}

impl ILifecycle for CounterCase {}

impl CounterCase {
    #[computed]
    pub fn counter_text(&self) -> String {
        format!("点击次数：{}", self.count)
    }

    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
    }
}

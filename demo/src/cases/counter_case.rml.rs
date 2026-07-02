use rml::prelude::*;

#[contribute(
    host_id = "demo.shell",
    id = "binding.counter",
    name = "case.counter.title",
    kind = "case",
    parent_id = "cat.binding",
    order = 1,
)]
#[component]
#[derive(Default)]
pub struct CounterCase {
    pub count: i32,
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

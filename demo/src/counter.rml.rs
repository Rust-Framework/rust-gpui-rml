use rml::prelude::*;

#[derive(IModel, Default)]
#[view]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count -= 1;
        cx.notify();
    }
}

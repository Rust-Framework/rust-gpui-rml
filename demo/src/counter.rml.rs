use rml::prelude::*;

#[window(title = "RML Counter Demo", width = 400, height = 500)]
#[derive(Default)]
pub struct Counter {
    pub count: i32,
    pub hovered: bool,
}

impl Counter {
    #[computed]
    pub fn double_count(&self) -> i32 {
        self.count * 2
    }

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

    #[command]
    pub fn on_hover_change(&mut self, ev: &HoverEvent, cx: &mut Context<Self>) {
        self.hovered = ev.is_hovering;
        cx.notify();
    }
}

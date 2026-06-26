use rml::prelude::*;

#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}

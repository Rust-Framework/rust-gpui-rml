use gpui::WeakEntity;
use rml::prelude::*;
use rml_core::window::IWindow;

use crate::shell::MainWindow;

#[window]
#[derive(Default)]
pub struct LoginWindow {
    pub username: String,
}

impl ILifecycle for LoginWindow {}

impl LoginWindow {
    #[command]
    pub fn on_login(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.username.trim().is_empty() {
            return;
        }
        let login: WeakEntity<LoginWindow> = cx.weak_entity();
        MainWindow::default().open(cx);
        cx.defer(move |cx| {
            if let Some(entity) = login.upgrade() {
                entity.update(cx, |win, cx| win.close(cx));
            }
        });
    }
}

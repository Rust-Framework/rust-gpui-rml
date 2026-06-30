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
        // 登录成功：先打开主窗口，再关闭登录窗
        // 顺序很重要：先 open 再 close，避免窗口全部退出导致应用终止
        MainWindow::default().open(cx);
        self.close(cx);
    }
}

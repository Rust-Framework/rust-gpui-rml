use rml::prelude::*;

#[window]
#[derive(Default)]
pub struct LoginDialog {
    pub username: String,
}

impl LoginDialog {
    #[command]
    pub fn on_login(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.username.trim().is_empty() {
            return;
        }
        // 登录成功：关闭对话框
        self.close(cx);
    }
}

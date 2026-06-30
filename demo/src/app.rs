//! 应用启动引导 —— 命令式入口，先登录再主窗口

use gpui::App;
use rml_app::{IAppLifecycle, RmlApplication};
use rml_core::i18n::I18nExt;
use rml_core::window::IWindow;

use crate::login::LoginWindow;

#[derive(Default)]
pub struct AppBootstrap;

impl IAppLifecycle for AppBootstrap {
    fn on_launch(&mut self, cx: &mut App) {
        // demo 资源位于 demo/assets/i18n（cargo run 时 cwd 为 workspace 根目录）
        cx.use_i18n_with_dir("zh-CN", "demo/assets/i18n");
        LoginWindow::default().open(cx);
    }
}

/// 命令式启动：登录窗 → 主窗口
pub fn run() {
    RmlApplication::new().run::<AppBootstrap>();
}

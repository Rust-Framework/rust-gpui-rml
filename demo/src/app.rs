//! 应用启动引导 —— 命令式入口：on_launch 打开主窗口，主窗口就绪后弹出登录对话框

use gpui::App;
use rml_app::IAppLifecycle;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_core::window::IWindow;

use crate::login::LoginDialog;
use crate::shell::MainWindow;

#[derive(Default)]
pub struct Startup;

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        // 全局样式 CSS 的 :root 颜色变量作为基础颜色,主题颜色覆盖之
        cx.set_style("styles.css");
        // i18n 与主题均从嵌入资源加载(assets/i18n、assets/themes)
        cx.set_i18n("zh-CN");
        cx.set_theme("light");
        // 先打开主窗口(MainWindow),再在其 Root 层上模态弹出 LoginDialog
        let mut main = MainWindow::default();
        main.open(cx);
        if let Some(handle) = main.handle() {
            let _ = handle.update(cx, |_, window, cx| {
                LoginDialog::default().open(window, cx);
            });
        }
    }
}

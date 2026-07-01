//! 应用启动引导 —— 声明式入口：on_launch 仅做全局初始化，主窗口由框架管理

use gpui::App;
use rml_app::IAppLifecycle;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;

use crate::features;

#[derive(Default)]
pub struct Startup;

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        cx.set_style("styles.css");
        cx.set_i18n("zh-CN");
        cx.set_theme("light");
        features::ensure_hosts(cx);
        features::register_all(cx);
    }
}

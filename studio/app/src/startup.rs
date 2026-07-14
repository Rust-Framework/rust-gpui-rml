//! 应用启动引导 —— 配置应用级资源(样式 / i18n / 主题)。

use gpui::App;
use rml_app::IAppLifecycle;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;

/// 应用启动引导 —— 配置应用级资源(样式 / i18n / 主题)。
#[derive(Default)]
pub struct Startup;

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        cx.set_style("styles.css");
        cx.set_i18n("zh-CN");
        cx.set_theme("light");
    }
}

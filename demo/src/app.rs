//! 应用启动引导 —— 声明式入口：on_launch 仅做全局初始化，主窗口由框架管理

use gpui::App;
use rml_app::IAppLifecycle;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;

#[derive(Default)]
pub struct Startup;

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        // 全局样式 CSS 的 :root 颜色变量作为基础颜色,主题颜色覆盖之
        cx.set_style("styles.css");
        // i18n 与主题均从嵌入资源加载(assets/i18n、assets/themes)
        cx.set_i18n("zh-CN");
        cx.set_theme("light");
    }
}

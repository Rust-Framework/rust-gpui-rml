//! 应用启动引导 —— 声明式入口：on_launch 仅做全局初始化，主窗口由框架管理

use gpui::{App, BorrowAppContext};
use rml_app::IAppLifecycle;
use rml_app::contribution::ContributionRegistryGlobal;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;

use crate::shell::contributions;

#[derive(Default)]
pub struct Startup;

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        cx.set_style("styles.css");
        cx.set_i18n("zh-CN");
        cx.set_theme("light");

        cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
            global.0.ensure_host(contributions::SHELL_HOST);
        });

        // 应用级纯元数据（分类节点、菜单/状态栏无 struct 的条目）
        contributions::register_case_categories(cx);
        contributions::register_menu_entry(cx, "menu.theme_toggle", "menu.theme_toggle", 0);
        contributions::register_menu_entry(cx, "menu.lang_en", "menu.lang_en", 10);
        contributions::register_status_entry(cx, "status.ready", "shell.status_ready", 0);

        // 自动注册所有 `#[contribute]` 案例/面板组件（build.rs 扫描生成）
        crate::register_rml_contributions(cx);
    }
}

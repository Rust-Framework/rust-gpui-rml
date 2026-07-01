//! 应用启动引导 —— 声明式入口：on_launch 仅做全局初始化，主窗口由框架管理

use gpui::{App, BorrowAppContext};
use rml_app::IAppLifecycle;
use rml_app::contribution::ContributionRegistryGlobal;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;

use crate::cases::{button_case, counter_case, i18n_case, two_way_case};
use crate::shell::case_activity_panel;
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

        contributions::register_case_categories(cx);

        counter_case::__rml_register_countercase(cx);
        two_way_case::__rml_register_twowaycase(cx);
        button_case::__rml_register_buttoncase(cx);
        i18n_case::__rml_register_i18ncase(cx);

        case_activity_panel::__rml_register_caseactivitypanel(cx);

        contributions::register_menu_entry(cx, "menu.theme_toggle", "menu.theme_toggle", 0);
        contributions::register_menu_entry(cx, "menu.lang_en", "menu.lang_en", 10);

        contributions::register_status_entry(cx, "status.ready", "shell.status_ready", 0);
    }
}

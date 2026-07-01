//! 功能模块 —— 自注册贡献点

use gpui::{App, BorrowAppContext};
use rml_app::contribution::ContributionRegistryGlobal;

use crate::cases::{
    button_case, counter_case, i18n_case, menu_context_case, menu_custom_case, menu_dropdown_case,
    menu_editor_case, menu_features_case, two_way_case,
};
use crate::shell::{case_activity_panel, contributions};

/// 注册所有功能模块贡献
pub fn register_all(cx: &mut App) {
    contributions::register_case_categories(cx);

    counter_case::__rml_register_countercase(cx);
    two_way_case::__rml_register_twowaycase(cx);
    button_case::__rml_register_buttoncase(cx);
    i18n_case::__rml_register_i18ncase(cx);
    menu_context_case::__rml_register_menucontextcase(cx);
    menu_dropdown_case::__rml_register_menudropdowncase(cx);
    menu_editor_case::__rml_register_menueditorcase(cx);
    menu_features_case::__rml_register_menufeaturescase(cx);
    menu_custom_case::__rml_register_menucustomcase(cx);

    case_activity_panel::__rml_register_caseactivitypanel(cx);

    contributions::register_menu_entry(cx, "menu.theme_toggle", "menu.theme_toggle", 0);
    contributions::register_menu_entry(cx, "menu.lang_en", "menu.lang_en", 10);
    contributions::register_status_entry(cx, "status.ready", "shell.status_ready", 0);
}

/// Demo 应用在启动时预创建需监听变更的 host（可选；首次 register 也会自动创建）
pub fn ensure_hosts(cx: &mut App) {
    cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
        global.0.ensure_host(contributions::SHELL_HOST);
    });
}

use gpui::{Entity, Window};
use rml::prelude::*;

use crate::cases::{
    ButtonCase, CounterCase, I18nCase, MenuContextCase, MenuDropdownCase, MenuEditorCase,
    MenuFeaturesCase, MenuCustomCase, TwoWayCase, WelcomeCase,
};

/// 按 `active_case_id` 路由到已注册案例组件（替代 MainWindow 内联 if 链）
#[component]
#[derive(Default)]
pub struct CaseHost {
    pub active_case_id: String,
    welcome_case: Option<Entity<WelcomeCase>>,
    counter_case: Option<Entity<CounterCase>>,
    two_way_case: Option<Entity<TwoWayCase>>,
    button_case: Option<Entity<ButtonCase>>,
    i18n_case: Option<Entity<I18nCase>>,
    menu_context_case: Option<Entity<MenuContextCase>>,
    menu_dropdown_case: Option<Entity<MenuDropdownCase>>,
    menu_editor_case: Option<Entity<MenuEditorCase>>,
    menu_features_case: Option<Entity<MenuFeaturesCase>>,
    menu_custom_case: Option<Entity<MenuCustomCase>>,
}

impl ILifecycle for CaseHost {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.welcome_case
            .get_or_insert_with(|| cx.new(|_| WelcomeCase::default()));
        self.counter_case
            .get_or_insert_with(|| cx.new(|_| CounterCase::default()));
        self.two_way_case
            .get_or_insert_with(|| cx.new(|_| TwoWayCase::default()));
        self.button_case
            .get_or_insert_with(|| cx.new(|_| ButtonCase::default()));
        self.i18n_case
            .get_or_insert_with(|| cx.new(|_| I18nCase::default()));
        self.menu_context_case
            .get_or_insert_with(|| cx.new(|_| MenuContextCase::default()));
        self.menu_dropdown_case
            .get_or_insert_with(|| cx.new(|_| MenuDropdownCase::default()));
        self.menu_editor_case
            .get_or_insert_with(|| cx.new(|_| MenuEditorCase::default()));
        self.menu_features_case
            .get_or_insert_with(|| cx.new(|_| MenuFeaturesCase::default()));
        self.menu_custom_case
            .get_or_insert_with(|| cx.new(|_| MenuCustomCase::default()));
    }
}

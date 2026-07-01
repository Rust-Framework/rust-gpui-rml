use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gpui::{BorrowAppContext, Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityPanels, MenuItems, StatusBarItems, TabItem};

use crate::cases::{
    self, ButtonCase, CounterCase, I18nCase, MenuContextCase, MenuDropdownCase, MenuEditorCase,
    MenuFeaturesCase, MenuCustomCase, OpenTab, TwoWayCase, WelcomeCase,
};
use crate::shell::contributions::{self, SHELL_HOST};
use crate::shell::CaseActivityPanel;

static MAIN_WINDOW_WEAK: Mutex<Option<gpui::WeakEntity<MainWindow>>> = Mutex::new(None);

/// 案例激活桥接入口 —— 供 `CaseActivityPanel::on_case_activate` 转发调用。
pub fn activate_case(case_id: String, app: &mut gpui::App) {
    if let Ok(guard) = MAIN_WINDOW_WEAK.lock() {
        if let Some(weak) = guard.as_ref() {
            if let Some(entity) = weak.upgrade() {
                entity.update(app, |main, cx| {
                    main.open_case(case_id, cx);
                });
            }
        }
    }
}

#[window]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    active_panel_id: String,
    activity_panels: ActivityPanels,
    status_items: StatusBarItems,
    i18n_version: u32,
    case_activity_panel: Option<Entity<CaseActivityPanel>>,
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
    menu_items: MenuItems,
    menu_commands: HashMap<String, Arc<dyn ICommand>>,
}


impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_tabs.is_empty() {
            self.open_tabs.push(OpenTab {
                id: "welcome".to_string(),
                title: cx.t("shell.welcome").to_string(),
            });
            self.selected_tab = 0;
            self.active_case_id = "welcome".to_string();
        }
        self.show_chrome = true;
        if self.active_panel_id.is_empty() {
            self.active_panel_id = "samples".to_string();
        }
        self.i18n_version = self.i18n_version.wrapping_add(1);

        self.case_activity_panel
            .get_or_insert_with(|| cx.new(|_| CaseActivityPanel::default()));
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

        self.menu_commands.insert(
            "menu.theme_toggle".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| this.apply_toggle_theme(cx))),
        );
        self.menu_commands.insert(
            "menu.lang_en".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| this.apply_switch_en(cx))),
        );

        if let Ok(mut guard) = MAIN_WINDOW_WEAK.lock() {
            *guard = Some(cx.weak_entity());
        }

        Self::wire_host_changed(cx);
        self.refresh_bindings(cx);

        window.defer(cx, |window, cx| {
            super::LoginDialog::default().open(window, cx);
        });
    }
}

impl MainWindow {
    fn wire_host_changed(cx: &mut Context<Self>) {
        let weak = cx.weak_entity();
        cx.update_global::<rml_app::contribution::ContributionRegistryGlobal, _>(|global, _| {
            global.0.set_host_on_changed(
                SHELL_HOST,
                Box::new(move |app| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(app, |main, cx| {
                            main.refresh_bindings(cx);
                        });
                    }
                }),
            );
        });
    }

    fn refresh_bindings(&mut self, cx: &mut Context<Self>) {
        self.activity_panels = contributions::build_activity_panels(cx, &self.active_panel_id);
        self.status_items = contributions::build_status_items(cx);
        self.menu_items = contributions::build_menu_items(cx, &self.menu_commands);
    }

    #[computed]
    pub fn tab_bar_items(&self) -> Vec<TabItem> {
        let _ = self.i18n_version;
        self.open_tabs
            .iter()
            .map(|tab| TabItem::new(tab.title.as_str()))
            .collect()
    }

    #[command]
    pub fn on_chrome_toggle(&mut self, cx: &mut Context<Self>) {
        self.show_chrome = !self.show_chrome;
    }

    #[command]
    pub fn on_panel_change(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let new_id = id.to_string();
        if self.active_panel_id == new_id {
            self.active_panel_id = String::new();
        } else {
            self.active_panel_id = new_id;
        }
        self.refresh_bindings(cx);
    }

    #[command]
    pub fn open_case(&mut self, case_id: String, cx: &mut Context<Self>) {
        if case_id.starts_with("cat.") {
            return;
        }
        if !self.open_tabs.iter().any(|tab| tab.id == case_id) {
            let tab = OpenTab {
                id: case_id.clone(),
                title: cx.t(cases::case_title_key(&case_id)).to_string(),
            };
            let mut tabs = std::mem::take(&mut self.open_tabs);
            tabs.push(tab);
            self.open_tabs = tabs;
        }
        self.selected_tab = self
            .open_tabs
            .iter()
            .position(|tab| tab.id == case_id)
            .unwrap_or(0);
        self.active_case_id = case_id;
    }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(tab) = self.open_tabs.get(index) {
            self.selected_tab = index;
            self.active_case_id = tab.id.clone();
        }
    }

    fn apply_toggle_theme(&mut self, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" {
            "light"
        } else {
            "dark"
        };
        cx.set_theme(next);
        self.i18n_version = self.i18n_version.wrapping_add(1);
        cx.notify();
    }

    fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
        cx.set_i18n("en-US");
        self.i18n_version = self.i18n_version.wrapping_add(1);
        self.open_tabs.iter_mut().for_each(|tab| {
            tab.title = cx.t(cases::case_title_key(&tab.id)).to_string();
        });
        self.refresh_bindings(cx);
        cx.notify();
    }
}

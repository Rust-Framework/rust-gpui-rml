use std::sync::Arc;

use gpui::{BorrowAppContext, Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityPanels, MenuItem, MenuItems, StatusBarItems, TabItem, TreeState};

use crate::cases::{self, ButtonCase, CounterCase, I18nCase, OpenTab, TwoWayCase, WelcomeCase};
use crate::features::navigation;
use crate::shell::{bindings, hosts};

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
    case_tree_state: Option<Entity<TreeState>>,
    i18n_version: u32,
    welcome_case: Option<Entity<WelcomeCase>>,
    counter_case: Option<Entity<CounterCase>>,
    two_way_case: Option<Entity<TwoWayCase>>,
    button_case: Option<Entity<ButtonCase>>,
    i18n_case: Option<Entity<I18nCase>>,
    menu_items: MenuItems,
    theme_cmd: Option<Arc<dyn ICommand>>,
    lang_cmd: Option<Arc<dyn ICommand>>,
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
        if self.case_tree_state.is_none() {
            self.case_tree_state = Some(cases::init_tree_state(cx));
        }
        self.i18n_version = self.i18n_version.wrapping_add(1);

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

        self.theme_cmd = Some(Arc::new(RelayCommand::new(cx, |this, cx| {
            this.apply_toggle_theme(cx)
        })));
        self.lang_cmd = Some(Arc::new(RelayCommand::new(cx, |this, cx| {
            this.apply_switch_en(cx)
        })));
        self.rebuild_menu_items(cx);

        let weak = cx.weak_entity();
        navigation::set_case_activate_handler(move |case_id, cx| {
            if let Some(entity) = weak.upgrade() {
                entity.update(cx, |main, cx| {
                    main.open_case(case_id, cx);
                });
            }
        });

        Self::wire_contribution_sync(cx);
        self.refresh_shell_bindings(cx);

        window.defer(cx, |window, cx| {
            super::LoginDialog::default().open(window, cx);
        });
    }
}

impl MainWindow {
    fn wire_contribution_sync(cx: &mut Context<Self>) {
        let weak = cx.weak_entity();
        cx.update_global::<rml_app::contribution::ContributionRegistryGlobal, _>(|global, _cx| {
            for host_id in [hosts::ACTIVITY_BAR, hosts::STATUS, hosts::CASE_TREE] {
                let weak = weak.clone();
                global.0.set_host_on_changed(
                    host_id,
                    Box::new(move |app| {
                        if let Some(entity) = weak.upgrade() {
                            entity.update(app, |main, cx| {
                                main.refresh_shell_bindings(cx);
                            });
                        }
                    }),
                );
            }
        });
    }

    fn refresh_shell_bindings(&mut self, cx: &mut Context<Self>) {
        self.activity_panels =
            bindings::activity_panels_from_host(cx, hosts::ACTIVITY_BAR, &self.active_panel_id);
        self.status_items = bindings::status_items_from_host(cx, hosts::STATUS);
    }

    fn rebuild_menu_items(&mut self, cx: &mut Context<Self>) {
        let theme_cmd = self
            .theme_cmd
            .clone()
            .expect("init theme_cmd in on_loaded");
        let lang_cmd = self.lang_cmd.clone().expect("init lang_cmd in on_loaded");
        self.menu_items = vec![
            MenuItem::new(cx.t("menu.theme_toggle"))
                .command(theme_cmd)
                .into_arc(),
            MenuItem::new(cx.t("menu.lang_en"))
                .command(lang_cmd)
                .into_arc(),
        ];
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
        self.refresh_shell_bindings(cx);
    }

    #[command]
    pub fn on_case_activate(&mut self, item_id: &SharedString, cx: &mut Context<Self>) {
        navigation::activate_case(item_id.to_string(), cx);
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
        if let Some(tree) = self.case_tree_state.as_ref() {
            cases::refresh_tree_state(tree, cx);
        }
        self.refresh_shell_bindings(cx);
        self.rebuild_menu_items(cx);
        cx.notify();
    }
}

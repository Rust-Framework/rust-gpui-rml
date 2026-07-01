use gpui::{BorrowAppContext, Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityPanels, StatusBarItems, TabItem, TreeState};

use crate::cases::{self, OpenTab};
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
    count: i32,
    pub name: String,
    #[validate(range(min = 0, max = 150))]
    pub age: i32,
    button_clicks: i32,
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

    #[computed]
    pub fn tab_bar_items(&self) -> Vec<TabItem> {
        let _ = self.i18n_version;
        self.open_tabs
            .iter()
            .map(|tab| TabItem::new(tab.title.as_str()))
            .collect()
    }

    #[computed]
    pub fn counter_text(&self) -> String {
        format!("点击次数：{}", self.count)
    }

    #[computed]
    pub fn button_demo_text(&self) -> String {
        format!("按钮点击：{}", self.button_clicks)
    }

    #[computed]
    pub fn profile_summary(&self) -> String {
        if self.name.is_empty() {
            format!("请输入姓名（年龄：{}）", self.age)
        } else {
            format!("你好，{}（{}岁）", self.name, self.age)
        }
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

    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
    }

    #[command]
    pub fn on_button_demo_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.button_clicks += 1;
    }

    #[command]
    pub fn on_switch_en(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        cx.set_i18n("en-US");
        self.i18n_version = self.i18n_version.wrapping_add(1);
        self.open_tabs.iter_mut().for_each(|tab| {
            tab.title = cx.t(cases::case_title_key(&tab.id)).to_string();
        });
        cases::refresh_tree_state(self.case_tree_state.as_ref().unwrap(), cx);
        self.refresh_shell_bindings(cx);
    }

    #[command]
    pub fn on_toggle_theme(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" {
            "light"
        } else {
            "dark"
        };
        cx.set_theme(next);
        self.i18n_version = self.i18n_version.wrapping_add(1);
    }
}

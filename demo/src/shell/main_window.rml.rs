use gpui::{Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::i18n::{t_static, I18nExt};
use rml_core::theme::ThemeExt;
use rml_ui::{
    ActivityPanel, ActivityPanels, IconName, TabItem, TreeState,
};

use crate::cases::{self, OpenTab};

#[window]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    /// 当前激活的活动栏面板 id（"samples" / 未来扩展）
    active_panel_id: String,
    i18n_version: u32,
    count: i32,
    pub name: String,
    #[validate(range(min = 0, max = 150))]
    pub age: i32,
    button_clicks: i32,
    case_tree_state: Option<Entity<TreeState>>,
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.case_tree_state.is_none() {
            self.case_tree_state = Some(cases::init_tree_state(cx));
        }
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
        // 弹出登录对话框（defer 到渲染周期外，避免 Root entity 借用冲突）
        window.defer(cx, |window, cx| {
            super::LoginDialog::default().open(window, cx);
        });
    }
}

impl MainWindow {
    #[computed]
    pub fn tab_bar_items(&self) -> Vec<TabItem> {
        let _ = self.i18n_version;
        self.open_tabs
            .iter()
            .map(|tab| TabItem::new(tab.title.as_str()))
            .collect()
    }

    #[computed]
    pub fn activity_icons(&self) -> ActivityPanels {
        let _ = self.i18n_version;
        let active_id = self.active_panel_id.clone();
        vec![
            ActivityPanel::new("samples", IconName::BookOpen, t_static("shell.samples"))
                .active(active_id == "samples")
                .into_arc(),
        ]
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
        cx.notify();
    }

    #[command]
    pub fn on_panel_change(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let new_id = id.to_string();
        if self.active_panel_id != new_id {
            self.active_panel_id = new_id;
            cx.notify();
        }
    }

    #[command]
    pub fn open_case(&mut self, case_id: String, cx: &mut Context<Self>) {
        if case_id.starts_with("cat.") {
            return;
        }
        if !self.open_tabs.iter().any(|tab| tab.id == case_id) {
            self.open_tabs.push(OpenTab {
                id: case_id.clone(),
                title: cx.t(cases::case_title_key(&case_id)).to_string(),
            });
        }
        self.selected_tab = self
            .open_tabs
            .iter()
            .position(|tab| tab.id == case_id)
            .unwrap_or(0);
        self.active_case_id = case_id;
        cx.notify();
    }

    #[command]
    pub fn on_case_activate(&mut self, item_id: &SharedString, cx: &mut Context<Self>) {
        self.open_case(item_id.to_string(), cx);
    }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(tab) = self.open_tabs.get(index) {
            self.selected_tab = index;
            self.active_case_id = tab.id.clone();
            cx.notify();
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
        if let Some(state) = &self.case_tree_state {
            cases::refresh_tree_state(state, cx);
        }
        self.open_tabs.iter_mut().for_each(|tab| {
            tab.title = cx.t(cases::case_title_key(&tab.id)).to_string();
        });
        cx.notify();
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
        cx.notify();
    }
}

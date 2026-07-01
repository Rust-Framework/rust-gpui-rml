use std::collections::HashMap;
use std::sync::Arc;

use gpui::{BorrowAppContext, Global, WeakEntity, Window};
use rml::prelude::*;
use crate::shell::shell_chrome::{map_shell_chrome, ShellChromeBindings};
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityPanels, MenuItems, StatusBarItems, TabItem};

use crate::cases::{self, OpenTab};
use crate::shell::case_activity_panel::CaseActivityPanel;
use crate::shell::case_host::CaseHost;
use rml_app::contribution::ContributionRegistryGlobal;
use rml_core::contribution::ComponentEntityCache;

/// Demo：Activity 视觉贡献面板回调 Host 开 Tab（由 MainWindow 在 `on_loaded` 注册）。
pub struct DemoShellHost(pub WeakEntity<MainWindow>);

impl Global for DemoShellHost {}

#[contributehost(id = "demo.shell", bindings = "refresh_bindings")]
#[window]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    activity_panels: ActivityPanels,
    status_items: StatusBarItems,
    i18n_version: u32,
    case_host: Option<gpui::Entity<CaseHost>>,
    menu_items: MenuItems,
    menu_commands: HashMap<String, Arc<dyn ICommand>>,
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.open_tabs.is_empty() {
            self.open_tabs.push(OpenTab {
                id: "welcome".to_string(),
                title: cx.t("shell.welcome").to_string(),
            });
            self.selected_tab = 0;
            self.active_case_id = "welcome".to_string();
        }
        self.show_chrome = true;
        self.i18n_version = self.i18n_version.wrapping_add(1);

        let shell_weak = cx.weak_entity();
        cx.set_global(DemoShellHost(shell_weak));

        self.case_host.get_or_insert_with(|| {
            let id = self.active_case_id.clone();
            cx.new(move |_| {
                let mut host = CaseHost::default();
                host.active_case_id = id;
                host
            })
        });

        self.menu_commands.insert(
            "menu.file.new".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| {
                this.open_case("welcome".to_string(), cx);
            })),
        );
        self.menu_commands.insert(
            "menu.file.open".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| {
                this.open_case("components.button".to_string(), cx);
            })),
        );
        self.menu_commands.insert(
            "menu.file.exit".to_string(),
            Arc::new(RelayCommand::action(|cx| {
                cx.quit();
            })),
        );
        self.menu_commands.insert(
            "menu.theme_toggle".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| this.apply_toggle_theme(cx))),
        );
        self.menu_commands.insert(
            "menu.lang_en".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| this.apply_switch_en(cx))),
        );
        self.menu_commands.insert(
            "menu.help.guide".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| {
                this.open_case("components.menu.dropdown".to_string(), cx);
            })),
        );
        self.menu_commands.insert(
            "menu.help.about".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| {
                this.open_case("welcome".to_string(), cx);
            })),
        );
        self.menu_commands.insert(
            "menu.open_features".to_string(),
            Arc::new(RelayCommand::new(cx, |this, cx| {
                this.open_case("components.menu.features".to_string(), cx);
            })),
        );

        let panel = cx.new(|_| CaseActivityPanel::default());
        cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
            global.0.entity_cache_mut().pre_register("samples", panel);
        });

        self.refresh_bindings(cx);
    }
}

impl MainWindow {
    fn refresh_bindings(&mut self, cx: &mut Context<Self>) {
        let ShellChromeBindings {
            activity_panels,
            status_items,
            menu_items,
        } = map_shell_chrome(Self::ID, cx, &self.menu_commands);
        self.activity_panels = activity_panels;
        self.status_items = status_items;
        self.menu_items = menu_items;
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
    pub fn on_panel_change(&mut self, _panel_id: &gpui::SharedString, cx: &mut Context<Self>) {
        cx.notify();
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
        self.active_case_id = case_id.clone();
        if let Some(host) = self.case_host.as_ref() {
            host.update(cx, |h, _| h.active_case_id = case_id);
        }
    }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(tab) = self.open_tabs.get(index) {
            self.selected_tab = index;
            self.active_case_id = tab.id.clone();
            if let Some(host) = self.case_host.as_ref() {
                host.update(cx, |h, _| h.active_case_id = tab.id.clone());
            }
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
        let mut tabs = std::mem::take(&mut self.open_tabs);
        tabs.iter_mut().for_each(|tab| {
            tab.title = cx.t(cases::case_title_key(&tab.id)).to_string();
        });
        self.open_tabs = tabs;
        self.refresh_bindings(cx);
        cx.notify();
    }
}

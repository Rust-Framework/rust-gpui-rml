use std::collections::HashMap;
use std::sync::Arc;

use gpui::{BorrowAppContext, Global, WeakEntity, Window};
use rml::prelude::*;
use crate::shell::shell_chrome::{map_shell_chrome, ShellChromeBindings};
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{
    ActivityBar, ActivityBarEvent, ActivityPanels, ActivitySidePanel, MenuItems, StatusBarItems,
    TabItem,
};

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
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    side_panel: Option<gpui::Entity<ActivitySidePanel>>,
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

        // 构造 ActivityBar + ActivitySidePanel 双 Entity（在 on_loaded 中，非 render）
        let panels = self.activity_panels.clone();
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels.clone())));
        self.side_panel = Some(cx.new(|_| ActivitySidePanel::new(panels)));

        // 订阅 ActivityBar 事件 → 联动 SidePanel
        // 必须在 activate_first 之前注册，否则初始 ItemActivated 事件会丢失
        if let Some(bar) = self.activity_bar.clone() {
            cx.subscribe(&bar, |this, _emitter, event: &ActivityBarEvent, cx| {
                if let Some(panel) = &this.side_panel {
                    match event {
                        ActivityBarEvent::ItemActivated(id) => {
                            panel.update(cx, |p, cx| p.set_active_id(Some(id.clone()), cx));
                        }
                        ActivityBarEvent::ItemDeactivated(_) => {
                            panel.update(cx, |p, cx| p.set_active_id(None, cx));
                        }
                    }
                }
            })
            .detach();
        }

        // subscribe 之后再激活首项，确保初始 ItemActivated 事件能被订阅者收到。
        // 同时直接设置 SidePanel 的 active_id —— 不依赖事件传递时序，
        // 保证首次 render 时 SidePanel 即有正确 active_id（事件到达后 subscriber 调用为 no-op）。
        let first_id = self.activity_panels.first().map(|p| p.id());
        if let Some(panel) = &self.side_panel {
            panel.update(cx, |p, cx| p.set_active_id(first_id.clone(), cx));
        }
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.activate_first(cx));
        }
    }
}

impl MainWindow {
    fn refresh_bindings(&mut self, cx: &mut Context<Self>) {
        let ShellChromeBindings {
            activity_panels,
            status_items,
            menu_items,
        } = map_shell_chrome(Self::ID, cx, &self.menu_commands);
        self.activity_panels = activity_panels.clone();
        self.status_items = status_items;
        self.menu_items = menu_items;

        // 同步面板数据到 ActivityBar + ActivitySidePanel Entity
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.set_panels(activity_panels.clone(), cx));
        }
        if let Some(panel) = &self.side_panel {
            panel.update(cx, |panel, cx| panel.set_panels(activity_panels, cx));
        }
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

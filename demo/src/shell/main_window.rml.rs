use std::collections::HashMap;
use std::sync::Arc;

use gpui::{BorrowAppContext, Global, IntoElement, WeakEntity, Window};
use rml::prelude::*;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityBar, MenuItems, StatusBarItems, TabItem};

use crate::cases::{self, OpenTab};
use crate::shell::activity_panel::ActivityPanel;
use crate::shell::shell_chrome::{map_menu_items, map_status_items};

/// Demo：ActivityPanel 通过它回调 MainWindow::open_case（在 `host_on_loaded` 中注册）。
pub struct DemoShellHost(pub WeakEntity<MainWindow>);

impl Global for DemoShellHost {}

/// MainWindow：`demo.shell` host + 视觉消费者。
///
/// `#[window]` + `#[contributehost]` 叠加：宏自动注入 `entries`/`i18n_version` 字段 +
/// 生成 `IContributionHost`/`ILifecycle` impl。业务代码只实现 `IHostEntity` 钩子。
#[window]
#[contributehost(id = "demo.shell")]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    status_items: StatusBarItems,
    menu_items: MenuItems,
    menu_commands: HashMap<String, Arc<dyn ICommand>>,
    slot_left_size: gpui::Pixels,
    // entries, i18n_version ← #[contributehost] 自动注入
}

// 无 impl IContributionHost —— #[contributehost] 宏自动生成
// 无 impl ILifecycle —— #[contributehost] 宏自动生成

impl IHostEntity for MainWindow {
    fn host_on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.open_tabs.is_empty() {
            self.open_tabs.push(OpenTab {
                id: "welcome".to_string(),
                title: cx.t("shell.welcome").to_string(),
            });
            self.selected_tab = 0;
            self.active_case_id = "welcome".to_string();
        }
        self.show_chrome = true;

        let shell_weak = cx.weak_entity();
        cx.set_global(DemoShellHost(shell_weak));

        // menu_commands 初始化
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

        // 刷新 shell chrome（从 self.entries 构建 menu/status items）
        self.refresh_shell_chrome();

        // 构建 ActivityBar：从 entries 中 kind="activity" 的视觉贡献自动提取
        let panels = rml_app::contribution::build_activity_panels(&*self.entries.read());
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels)));

        // observe ActivityPanel Entity（框架缓存）→ ActivityBar 重渲
        let panel_entity = rml_app::contribution::visual_entity::<ActivityPanel>(cx);
        cx.observe(&panel_entity, |_, _, cx| cx.notify()).detach();

        // 激活首项
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.activate_first(cx));
        }

        self.slot_left_size = gpui::px(260.);

        // observe ActivityBar active_id 变化 → 同步 slot_left_size
        if let Some(bar) = &self.activity_bar {
            cx.observe(bar, |this, bar, cx| {
                let collapsed = bar.read(cx).active_id().is_none();
                this.slot_left_size = if collapsed {
                    gpui::px(48.)
                } else {
                    gpui::px(260.)
                };
                cx.notify();
            })
            .detach();
        }
    }
}

impl MainWindow {
    fn refresh_shell_chrome(&mut self) {
        let entries = self.entries.read();
        self.status_items = map_status_items(&entries);
        self.menu_items = map_menu_items(&entries, &self.menu_commands);
    }

    /// 渲染当前激活的 IVisualContribution 视图。
    /// Entity 由框架实体缓存复用，状态持久。
    pub fn active_case_view(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::AnyElement {
        use rml_app::contribution::extract_visual;
        let entries = self.entries.read();
        if let Some(entry) = entries
            .iter()
            .find(|e| e.contribution.id() == self.active_case_id)
        {
            if let Some(visual) = extract_visual(&entry.contribution) {
                return visual.render(window, cx);
            }
        }
        gpui::div().into_any_element()
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
        if case_id.starts_with("group.") {
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
        cx.notify();
    }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(tab) = self.open_tabs.get(index) {
            self.selected_tab = index;
            self.active_case_id = tab.id.clone();
            cx.notify();
        }
    }

    fn apply_toggle_theme(&mut self, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" {
            "light"
        } else {
            "dark"
        };
        cx.set_theme(next);
        cx.notify();
    }

    fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
        cx.set_i18n("en-US");
        // I18nState 变化触发 observe_global → 自动 bump i18n_version + on_locale_changed
        // 手动刷新 tab 标题（on_locale_changed 不处理 tab 标题）
        let mut tabs = std::mem::take(&mut self.open_tabs);
        tabs.iter_mut().for_each(|tab| {
            tab.title = cx.t(cases::case_title_key(&tab.id)).to_string();
        });
        self.open_tabs = tabs;
        self.refresh_shell_chrome();
        cx.notify();
    }
}

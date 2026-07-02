use std::collections::HashMap;
use std::sync::Arc;

use gpui::{BorrowAppContext, Global, WeakEntity, Window};
use rml::prelude::*;
use crate::shell::shell_chrome::{map_shell_chrome, ShellChromeBindings};
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{
    ActivityBar, ActivityPanels, MenuItems, StatusBarItems, TabItem,
};

use crate::cases::{self, OpenTab};
use crate::shell::activity_panel::ActivityPanel;
use rml_app::contribution::{subscribe_host_changes, ContributionRegistryGlobal};
use rml_core::contribution::ComponentEntityCache;

/// Demo：Activity 视觉贡献面板回调 Host 开 Tab（由 MainWindow 在 `on_loaded` 注册）。
pub struct DemoShellHost(pub WeakEntity<MainWindow>);

impl Global for DemoShellHost {}

#[contributehost(id = "demo.shell")]
#[window]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    activity_panels: ActivityPanels,
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    status_items: StatusBarItems,
    i18n_version: u32,
    menu_items: MenuItems,
    menu_commands: HashMap<String, Arc<dyn ICommand>>,
    /// 左侧插槽当前宽度。由 `cx.observe(&activity_bar)` 同步：
    /// ActivityBar 收起（active_id=None）→ 48px（仅图标栏），展开 → 260px。
    /// 私有字段：避免被 codegen 生成 InputState 双向绑定（Pixels 非 SharedString）。
    /// RML 通过 `left_size={slot_left_size}` 直接引用，生成 `self.slot_left_size`。
    slot_left_size: gpui::Pixels,
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

        let panel = cx.new(|_| ActivityPanel::default());
        cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
            global.0.entity_cache_mut().pre_register("samples", panel);
        });

        self.refresh_shell_chrome(cx);

        // 构造 ActivityBar 单 Entity（在 on_loaded 中，非 render）
        let panels = self.activity_panels.clone();
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels)));

        // 激活首项 —— 单 Entity 内 set_active_id 直接 cx.notify() 触发重渲
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.activate_first(cx));
        }

        // 初始展开态：与 activate_first 后的 active_id=Some 一致
        self.slot_left_size = gpui::px(260.);

        // 监听 ActivityBar active_id 变化，同步 slot_left_size：
        // 收起（active_id=None）→ 48px（仅图标栏），展开 → 260px。
        // observe 注册后不会立即触发，故上面先手动初始化。
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

        // 订阅 host 贡献变更：贡献点注册/注销时自动刷新 shell chrome
        subscribe_host_changes(Self::ID, cx, |this, cx| {
            this.refresh_shell_chrome(cx);
            cx.notify();
        });
    }
}

impl rml_core::contribution::IContributionHost for MainWindow {
    const ID: &'static str = "demo.shell";
}

impl MainWindow {
    fn refresh_shell_chrome(&mut self, cx: &mut Context<Self>) {
        let ShellChromeBindings {
            activity_panels,
            status_items,
            menu_items,
        } = map_shell_chrome(Self::ID, cx, &self.menu_commands);
        self.activity_panels = activity_panels.clone();
        self.status_items = status_items;
        self.menu_items = menu_items;

        // 同步面板数据到 ActivityBar Entity
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.set_panels(activity_panels, cx));
        }
    }

    /// 渲染当前激活的 IVisualContribution 视图（供 RML 模板 `content={...}` 调用）。
    /// 从 `ContributionRegistry` 查找 `active_case_id` 对应条目，委托给
    /// `render_contribution_visual` 执行视觉渲染（内部复用 Entity 缓存）。
    pub fn active_case_view(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::AnyElement {
        use rml_app::contribution::{contribution_entries, render_contribution_visual};
        use rml_core::contribution::VisualRenderer;
        let visual: Option<VisualRenderer> = {
            let entries = contribution_entries(Self::ID, cx);
            entries
                .iter()
                .find(|e| e.contribution.id() == self.active_case_id)
                .and_then(|e| e.visual.clone())
        };
        if let Some(visual) = visual {
            return render_contribution_visual(&visual, window, cx)
                .unwrap_or_else(|| gpui::div().into_any_element());
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
        self.refresh_shell_chrome(cx);
        cx.notify();
    }
}

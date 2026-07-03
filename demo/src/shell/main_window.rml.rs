use std::sync::Arc;

use gpui::{BorrowAppContext, Global, IntoElement, WeakEntity, Window};
use rml::prelude::*;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityBar, IMenuItem, IStatusBarItem, TabItem};

use crate::cases::{self, OpenTab};
use crate::shell::activity_panel::ActivityPanel;
use crate::shell::shell_chrome::{
    build_activity_panels_from, map_menu_items, map_status_items, ContribEntry,
};

/// Demo：ActivityPanel 通过它回调 MainWindow::open_case（在 `on_loaded` 中注册）。
pub struct DemoShellHost(pub WeakEntity<MainWindow>);

impl Global for DemoShellHost {}

/// MainWindow：`demo.shell` host + 视觉消费者。
///
/// `#[window]` + `#[contributehost]` 叠加：用户手写 `impl IContributionHost`（override
/// `add`/`remove`）+ `impl ILifecycle`（在 `on_loaded` 中调
/// `__rml_install_host` + `drain_host_ops`）。宏仅生成 `pub const ID` + `__rml_install_host`。
///
/// 贡献存储单一桶 `entries`：所有贡献（menu/status/case/activity）由 `add` 受理，
/// 投影时经 `as_command()`/`as_visual()` 能力查询区分类型。
#[window]
#[contributehost(id = "demo.shell")]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    status_items: Vec<Arc<dyn IStatusBarItem>>,
    menu_items: Vec<Arc<dyn IMenuItem>>,
    slot_left_size: gpui::Pixels,
    // 单一存储桶：所有贡献（menu/status/case/activity）
    entries: std::sync::RwLock<Vec<ContribEntry>>,
    // host handle receiver（drain 在 on_loaded / refresh 中进行）
    host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
}

impl IContributionHost for MainWindow {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn add(&self, contribution: Arc<dyn IContribution>, options: Option<ContributionOptions>) {
        let opts = options.unwrap_or_default();
        self.entries.write().unwrap().push((contribution, opts));
    }

    fn remove(&self, contribution_id: &str) {
        self.entries
            .write()
            .unwrap()
            .retain(|(c, _)| c.id() != contribution_id);
    }
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 1. 注册 host handle + 触发 demo.shell 的所有贡献注册（同步入队）
        let rx = Self::__rml_install_host(cx.entity(), cx);
        self.host_rx = Some(rx);

        // 2. drain 队列中的 HostOp → 调用自身 IContributionHost::add
        if let Some(rx) = &self.host_rx {
            rml_app::contribution::drain_host_ops(rx, self);
        }

        // 3. 初始化 welcome tab / DemoShellHost / menu_commands
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

        // 4. 刷新 shell chrome（从 entries 构建 menu/status items）
        self.refresh_shell_chrome();

        // 5. 构建 ActivityBar：从 entries 中 slot="activity" 的视觉贡献
        let panels = {
            let entries = self.entries.read().unwrap();
            build_activity_panels_from(&entries)
        };
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

        cx.notify();
    }
}

impl MainWindow {
    fn refresh_shell_chrome(&mut self) {
        let entries = self.entries.read().unwrap();
        self.status_items = map_status_items(&entries);
        self.menu_items = map_menu_items(&entries);
    }

    /// 渲染当前激活的 IVisualContribution 视图。
    /// Entity 由框架实体缓存复用，状态持久。
    pub fn active_case_view(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::AnyElement {
        let entries = self.entries.read().unwrap();
        if let Some((c, _)) = entries
            .iter()
            .find(|(c, _)| c.id() == self.active_case_id)
        {
            if let Some(visual) = c.as_visual() {
                return visual.render(window, cx);
            }
        }
        gpui::div().into_any_element()
    }

    #[computed]
    pub fn tab_bar_items(&self) -> Vec<TabItem> {
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

    pub(crate) fn apply_toggle_theme(&mut self, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" {
            "light"
        } else {
            "dark"
        };
        cx.set_theme(next);
        cx.notify();
    }

    pub(crate) fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
        cx.set_i18n("en-US");
        // set_i18n 已触发 refresh_windows；手动刷新 tab 标题与 shell chrome
        let mut tabs = std::mem::take(&mut self.open_tabs);
        tabs.iter_mut().for_each(|tab| {
            tab.title = cx.t(cases::case_title_key(&tab.id)).to_string();
        });
        self.open_tabs = tabs;
        self.refresh_shell_chrome();
        cx.notify();
    }
}

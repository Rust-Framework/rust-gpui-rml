use std::collections::HashMap;
use std::sync::Arc;

use gpui::{BorrowAppContext, Global, IntoElement, WeakEntity, Window};
use rml::prelude::*;
use rml_core::contribution::{ContributionAbilityExt, IContribution, VisualAbilityExt};
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityBar, IMenuItem, IStatusBarItem};

use crate::cases::{self, OpenTab};
use crate::lsp::{CodeEditorTab, LspClient};
use crate::lsp::lsp_explorer_panel::LspExplorerPanel;
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
    open_tabs: Vec<Arc<dyn IValue>>,
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
    // LSP 子进程客户端
    lsp_client: Option<Arc<LspClient>>,
    // LSP 文件 Tab：key = "lsp://<relative_path>"
    lsp_tabs: HashMap<String, gpui::Entity<CodeEditorTab>>,
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
            self.open_tabs.push(Arc::new(OpenTab {
                id: "welcome".to_string(),
                title: cx.t("shell.welcome").to_string(),
            }) as Arc<dyn IValue>);
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

        // 6. observe LspExplorerPanel Entity（框架缓存）→ ActivityBar 重渲
        let lsp_panel_entity = rml_app::contribution::visual_entity::<LspExplorerPanel>(cx);
        cx.observe(&lsp_panel_entity, |_, _, cx| cx.notify())
            .detach();

        // 7. 启动 LSP 子进程（失败时优雅降级，demo 继续运行）
        if let Ok(workspace_root) = std::env::current_dir() {
            match LspClient::spawn(&workspace_root) {
                Ok(client) => {
                    self.lsp_client = Some(Arc::new(client));
                }
                Err(e) => {
                    log::warn!("Failed to start LSP server: {e}");
                }
            }
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
        // LSP 文件 Tab 分流：懒加载 CodeEditorTab
        if self.active_case_id.starts_with("lsp://") {
            let tab_id = self.active_case_id.clone();
            if !self.lsp_tabs.contains_key(&tab_id) {
                if let Some(client) = self.lsp_client.clone() {
                    if let Some(relative_path) = tab_id.strip_prefix("lsp://") {
                        let full_path = std::env::current_dir()
                            .unwrap_or_default()
                            .join("src")
                            .join(relative_path);
                        let tab = CodeEditorTab::new(
                            relative_path,
                            &full_path,
                            client,
                            window,
                            cx,
                        );
                        self.lsp_tabs.insert(tab_id.clone(), tab);
                    }
                }
            }
            if let Some(tab) = self.lsp_tabs.get(&tab_id) {
                return tab.update(cx, |tab, cx| tab.render(window, cx).into_any_element());
            }
            return gpui::div().into_any_element();
        }

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
    pub fn tab_bar_items(&self) -> Vec<Arc<dyn IValue>> {
        self.open_tabs.clone()
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
        if !self
            .open_tabs
            .iter()
            .any(|tab| tab.as_contribution().map(|c| c.id() == case_id).unwrap_or(false))
        {
            self.open_tabs.push(Arc::new(OpenTab {
                id: case_id.clone(),
                title: cx.t(cases::case_title_key(&case_id)).to_string(),
            }) as Arc<dyn IValue>);
        }
        self.selected_tab = self
            .open_tabs
            .iter()
            .position(|tab| tab.as_contribution().map(|c| c.id() == case_id).unwrap_or(false))
            .unwrap_or(0);
        self.active_case_id = case_id;
        cx.notify();
    }

    /// 由 LspExplorerPanel::on_file_activate 通过 DemoShellHost 回调。
    /// 仅注册 Tab 元信息；CodeEditorTab Entity 在 active_case_view 中懒加载。
    #[command]
    pub fn open_lsp_file(&mut self, relative_path: String, cx: &mut Context<Self>) {
        let tab_id = format!("lsp://{relative_path}");
        if !self
            .open_tabs
            .iter()
            .any(|tab| tab.as_contribution().map(|c| c.id() == tab_id).unwrap_or(false))
        {
            let title = std::path::Path::new(&relative_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&relative_path)
                .to_string();
            self.open_tabs.push(Arc::new(OpenTab {
                id: tab_id.clone(),
                title,
            }) as Arc<dyn IValue>);
        }
        self.selected_tab = self
            .open_tabs
            .iter()
            .position(|tab| tab.as_contribution().map(|c| c.id() == tab_id).unwrap_or(false))
            .unwrap_or(0);
        self.active_case_id = tab_id;
        cx.notify();
    }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(tab) = self.open_tabs.get(index) {
            self.selected_tab = index;
            self.active_case_id = tab
                .as_contribution()
                .map(|c| c.id().to_string())
                .unwrap_or_default();
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
        self.open_tabs = self
            .open_tabs
            .iter()
            .map(|tab| {
                let id = tab
                    .as_contribution()
                    .map(|c| c.id().to_string())
                    .unwrap_or_default();
                let title = cx.t(cases::case_title_key(&id)).to_string();
                Arc::new(OpenTab { id, title }) as Arc<dyn IValue>
            })
            .collect();
        self.refresh_shell_chrome();
        cx.notify();
    }
}

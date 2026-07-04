use std::sync::Arc;

use gpui::{IntoElement, WeakEntity, Window};
use rml::prelude::*;
use rml_app::IAppContextExt;
use rml_core::command::{ICommand, RelayCommand};
use rml_core::contribution::{IContribution, VisualAbilityExt};
use rml_core::i18n::{t_static, I18nExt};
use rml_core::theme::ThemeExt;
use rml_core::workbench::Uri;
use rml_ui::{ActivityBar, IActivityPanel, VisualActivityPanel};

use crate::lsp::LspClient;
use crate::lsp::lsp_explorer_panel::LspExplorerPanel;
use crate::shell::activity_panel::ActivityPanel;
use crate::shell::case_view_model::CaseViewModel;
use crate::shell::menu_view_model::MenuViewModel;
use crate::shell::status_view_model::{build_status_view_models, ContribEntry, StatusViewModel};
use crate::shell::workbench::DemoWorkbenchManager;

/// MainWindow 弱引用槽位——经 IAppContext::set_service 注册为单例，
/// ActivityPanel / LspExplorerPanel / 菜单命令通过 get_service::<MainWindowRef>() 查询。
pub struct MainWindowRef(pub WeakEntity<MainWindow>);

/// MainWindow：`demo.shell` host + ViewModel。
///
/// 持有 `cases` / `menus` / `status` / `activities` 四个类型化集合，
/// 直接绑定模板（tree / menu-bar / status-bar / ActivityBar）。
/// Tab/资源生命周期委托给 `DemoWorkbenchManager`。
///
/// 菜单改用 `RelayCommand` 字段（WPF MVVM 模式），消除 menu_shell_contribs.rs 样板。
#[window]
#[contributehost(id = "demo.shell")]
pub struct MainWindow {
    // 直接绑定模板的集合（on_loaded 后一次性填充）
    pub cases: Vec<CaseViewModel>,
    pub menus: Vec<MenuViewModel>,
    pub status: Vec<StatusViewModel>,
    activities: Vec<Arc<dyn IActivityPanel>>,

    // RelayCommand 字段（WPF MVVM 模式，7 个叶子命令）
    open_welcome_command: Arc<dyn ICommand>,
    open_button_case_command: Arc<dyn ICommand>,
    open_menu_dropdown_case_command: Arc<dyn ICommand>,
    open_features_case_command: Arc<dyn ICommand>,
    toggle_theme_command: Arc<dyn ICommand>,
    switch_en_command: Arc<dyn ICommand>,
    exit_command: Arc<dyn ICommand>,

    // Tab 状态（manager 派生缓存，命令后同步）
    open_tabs: Vec<Arc<dyn IValue>>,
    selected_tab: usize,
    show_chrome: bool,
    slot_left_size: gpui::Pixels,

    // 框架仪式
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    entries: std::sync::RwLock<Vec<ContribEntry>>,
    host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
    manager: Option<Arc<DemoWorkbenchManager>>,
    lsp_client: Option<Arc<LspClient>>,
}

/// 手写 `Default`——`Arc<dyn ICommand>` 无 `#[derive(Default)]`，
/// 用 `RelayCommand::default()`（no-op）初始化所有命令字段，`on_loaded` 中替换为真实命令。
/// `#[window]` 宏注入的版本计数器 / 缓存 / 状态字段全部用 `Default::default()` 初始化。
impl Default for MainWindow {
    fn default() -> Self {
        let default_cmd: Arc<dyn ICommand> = Arc::new(RelayCommand::default());
        Self {
            cases: Vec::new(),
            menus: Vec::new(),
            status: Vec::new(),
            activities: Vec::new(),
            open_welcome_command: default_cmd.clone(),
            open_button_case_command: default_cmd.clone(),
            open_menu_dropdown_case_command: default_cmd.clone(),
            open_features_case_command: default_cmd.clone(),
            toggle_theme_command: default_cmd.clone(),
            switch_en_command: default_cmd.clone(),
            exit_command: default_cmd,
            open_tabs: Vec::new(),
            selected_tab: 0,
            show_chrome: false,
            slot_left_size: gpui::px(260.),
            activity_bar: None,
            entries: std::sync::RwLock::new(Vec::new()),
            host_rx: None,
            manager: None,
            lsp_client: None,
            // #[window] 注入字段
            __rml_window_handle: None,
            // 版本计数器（每个字段一个，含 __rml_window_handle 自身）
            __rml_cases_version: Default::default(),
            __rml_menus_version: Default::default(),
            __rml_status_version: Default::default(),
            __rml_activities_version: Default::default(),
            __rml_open_welcome_command_version: Default::default(),
            __rml_open_button_case_command_version: Default::default(),
            __rml_open_menu_dropdown_case_command_version: Default::default(),
            __rml_open_features_case_command_version: Default::default(),
            __rml_toggle_theme_command_version: Default::default(),
            __rml_switch_en_command_version: Default::default(),
            __rml_exit_command_version: Default::default(),
            __rml_open_tabs_version: Default::default(),
            __rml_selected_tab_version: Default::default(),
            __rml_show_chrome_version: Default::default(),
            __rml_slot_left_size_version: Default::default(),
            __rml_activity_bar_version: Default::default(),
            __rml_entries_version: Default::default(),
            __rml_host_rx_version: Default::default(),
            __rml_manager_version: Default::default(),
            __rml_lsp_client_version: Default::default(),
            __rml___rml_window_handle_version: Default::default(),
            // #[component] 注入的缓存 / 状态字段
            __rml_computed_cache: Default::default(),
            __rml_input_states: Default::default(),
            __rml_input_state_versions: Default::default(),
            __rml_field_errors: Default::default(),
            __rml_loaded: false,
        }
    }
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
        // 1. 注册 host + drain（add 期间填充 entries 暂存）
        let rx = Self::__rml_install_host(cx.entity(), cx);
        self.host_rx = Some(rx);
        if let Some(rx) = &self.host_rx {
            rml_app::contribution::drain_host_ops(rx, self);
        }

        // 2. 初始化 RelayCommand 字段（WPF MVVM 模式）
        self.open_welcome_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.open_case("welcome".to_string(), cx);
        }));
        self.open_button_case_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.open_case("components.button".to_string(), cx);
        }));
        self.open_menu_dropdown_case_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.open_case("components.menu.dropdown".to_string(), cx);
        }));
        self.open_features_case_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.open_case("components.menu.features".to_string(), cx);
        }));
        self.toggle_theme_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.apply_toggle_theme(cx);
        }));
        self.switch_en_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.apply_switch_en(cx);
        }));
        self.exit_command = Arc::new(RelayCommand::action(|cx| cx.quit()));

        // 2.5 注册 StatusReady 视觉能力（project_entries 前完成，使 as_visual() 查询生效）
        crate::cases::status_bar_case::ensure_status_ready_registered();

        // 3. 投影到类型化集合（cases/status/activities 经贡献；menus 手工构建）
        self.project_entries();

        // 4. MainWindowRef 单例（ActivityPanel/LspExplorerPanel on_loaded 经 IAppContext 查询）
        let shell_weak = cx.weak_entity();
        cx.set_service(Arc::new(MainWindowRef(shell_weak)));

        // 5. 启动 LSP 子进程（失败时优雅降级）
        if let Ok(workspace_root) = std::env::current_dir() {
            match LspClient::spawn(&workspace_root) {
                Ok(client) => self.lsp_client = Some(Arc::new(client)),
                Err(e) => log::warn!("Failed to start LSP server: {e}"),
            }
        }

        // 6. 构建 manager + 安装 + 同步 cases 副本到 provider
        let manager = Arc::new(DemoWorkbenchManager::new(self.lsp_client.clone()));
        manager.sync_cases(self.cases.clone());
        cx.set_workbench_manager(manager.clone());
        self.manager = Some(manager);

        // 7. 打开 welcome tab（经 manager）
        if let Some(manager) = self.manager.clone() {
            let uri: Uri = "rml://welcome".parse().unwrap();
            manager.open(&uri);
            self.sync_tab_state(&manager);
        }

        // 8. 构建 ActivityBar（从 activities 集合）
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(self.activities.clone())));

        // observe ActivityPanel Entity（框架缓存）→ ActivityBar 重渲
        let panel_entity = rml_app::contribution::visual_entity::<ActivityPanel>(cx);
        cx.observe(&panel_entity, |_, _, cx| cx.notify())
            .detach();

        // 激活首项
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.activate_first(cx));
        }

        self.show_chrome = true;
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

        // 9. observe LspExplorerPanel Entity（框架缓存）→ ActivityBar 重渲
        let lsp_panel_entity = rml_app::contribution::visual_entity::<LspExplorerPanel>(cx);
        cx.observe(&lsp_panel_entity, |_, _, cx| cx.notify())
            .detach();

        cx.notify();
    }
}

impl MainWindow {
    /// 从 entries 暂存投影到 cases/status/activities 类型化集合。
    /// menus 不经贡献系统，由 `build_menu_tree` 手工构建。
    fn project_entries(&mut self) {
        let entries = self.entries.read().unwrap();
        self.cases = entries
            .iter()
            .filter_map(|(c, o)| CaseViewModel::from_contribution(c.clone(), o.clone()))
            .collect();
        self.status = build_status_view_models(&entries);
        self.activities = entries
            .iter()
            .filter(|(c, o)| o.effective_slot() == Some("activity") && c.as_visual().is_some())
            .filter_map(|(c, _)| {
                VisualActivityPanel::new(c.clone()).map(|p| Arc::new(p) as Arc<dyn IActivityPanel>)
            })
            .collect();
        // menus 不经贡献系统，由 build_menu_tree 手工构建
        self.menus = self.build_menu_tree();
    }

    /// 手工构建菜单树（消除 menu_shell_contribs.rs + shell_chrome.rs）。
    /// 标签经 `t_static()` 获取 i18n；命令绑定到 RelayCommand 字段。
    fn build_menu_tree(&self) -> Vec<MenuViewModel> {
        vec![
            MenuViewModel::root("menu.file", t_static("menu.file"), 0)
                .child(MenuViewModel::leaf(
                    "menu.file.new",
                    t_static("menu.file_new"),
                    0,
                    self.open_welcome_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    "menu.file.open",
                    t_static("menu.file_open"),
                    1,
                    self.open_button_case_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    "menu.file.exit",
                    t_static("menu.file_exit"),
                    2,
                    self.exit_command.clone(),
                )),
            MenuViewModel::root("menu.view", t_static("menu.view"), 10)
                .child(MenuViewModel::leaf(
                    "menu.theme_toggle",
                    t_static("menu.theme_toggle"),
                    0,
                    self.toggle_theme_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    "menu.lang_en",
                    t_static("menu.lang_en"),
                    1,
                    self.switch_en_command.clone(),
                )),
            MenuViewModel::root("menu.help", t_static("menu.help"), 20)
                .child(
                    MenuViewModel::root(
                        "menu.help.docs",
                        t_static("case.menu.help_center"),
                        0,
                    )
                    .child(MenuViewModel::leaf(
                        "menu.help.guide",
                        t_static("case.menu.nested"),
                        0,
                        self.open_menu_dropdown_case_command.clone(),
                    ))
                    .child(MenuViewModel::leaf(
                        "menu.help.about",
                        t_static("menu.help_about"),
                        1,
                        self.open_welcome_command.clone(),
                    )),
                )
                .child(
                    MenuViewModel::root(
                        "menu.help.cases",
                        t_static("case.menu.features.group"),
                        1,
                    )
                    .child(MenuViewModel::leaf(
                        "menu.open_features",
                        t_static("case.menu.features.title"),
                        0,
                        self.open_features_case_command.clone(),
                    )),
                ),
        ]
    }

    /// 同步 tab 状态：从 manager 派生 open_tabs + selected_tab。
    fn sync_tab_state(&mut self, manager: &DemoWorkbenchManager) {
        self.open_tabs = manager.get_all_as_values();
        self.selected_tab = manager.activated_index().unwrap_or(0);
    }

    /// 渲染激活的 workbench 视图（替代旧 active_case_view，LSP 分流由 manager 路由）。
    pub fn active_view(&self, window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        if let Some(manager) = &self.manager {
            if let Some(wb) = manager.get_activated_demo() {
                return wb.render(window, cx);
            }
        }
        gpui::div().into_any_element()
    }

    /// 渲染菜单栏（从 `menus` ViewModel 树构建 `MenuBar` + `PopupMenu`）。
    ///
    /// 子菜单经 `dropdown_menu` 闭包 + `MenuViewModel::build_popup_menu` 递归构建。
    /// `children` 在闭包外 clone 以满足 `'static` bound。
    pub fn render_menu_bar(
        &self,
        _window: &mut Window,
        _cx: &mut gpui::Context<Self>,
    ) -> gpui::AnyElement {
        use rml_ui::{MenuBar, configure_menu_bar_popup, menu_bar_button};
        use gpui_component::menu::{DropdownMenu as _, PopupMenu};
        use gpui::{ParentElement, Styled};

        if self.menus.is_empty() {
            return gpui::div().into_any_element();
        }

        let mut bar = MenuBar::new(("rml_menu_bar", 0usize));
        for (ix, m) in self.menus.iter().enumerate() {
            let btn: gpui::AnyElement = if m.has_children() {
                let children = m.children.clone();
                let label = m.label.clone();
                menu_bar_button(("rml_menu_btn", ix), label)
                    .dropdown_menu(move |menu, window, cx| {
                        let menu = configure_menu_bar_popup(menu);
                        MenuViewModel::build_popup_menu(menu, &children, window, cx)
                    })
                    .into_any_element()
            } else {
                let cmd = m.command.clone();
                let mut btn = menu_bar_button(("rml_menu_btn", ix), m.label.clone());
                if let Some(cmd) = cmd {
                    btn = btn.on_click(move |_, window, app| {
                        let mut ctx = rml_core::command::CallContext::new(window, app);
                        if cmd.can_execute(&mut ctx) {
                            cmd.execute(&mut ctx);
                        }
                    });
                }
                btn.into_any_element()
            };
            bar = bar.child(btn);
        }
        bar.into_any_element()
    }

    /// 渲染状态栏（从 `status` ViewModel 列表构建 `NativeStatusBar`）。
    pub fn render_status_bar(
        &self,
        window: &mut Window,
        _cx: &mut gpui::Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::ParentElement;
        use rml_ui::{NativeStatusBar, StatusBarAlign};

        let mut bar = NativeStatusBar::new();
        for s in &self.status {
            let content = s.render(window, _cx);
            match s.align {
                StatusBarAlign::Left => {
                    bar = bar.left(content);
                }
                StatusBarAlign::Right => {
                    bar = bar.right(content);
                }
                StatusBarAlign::Center => {
                    bar = bar.child(content);
                }
            }
        }
        bar.into_any_element()
    }

    #[computed]
    pub fn tab_bar_items(&self) -> Vec<Arc<dyn IValue>> {
        self.open_tabs.clone()
    }

    #[command]
    pub fn on_chrome_toggle(&mut self, _cx: &mut Context<Self>) {
        self.show_chrome = !self.show_chrome;
    }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(manager) = self.manager.clone() {
            manager.activate_by_index(index);
            self.sync_tab_state(&manager);
            cx.notify();
        }
    }

    /// 由 ActivityPanel::on_case_activate 调用（经 MainWindowRef 回调）。
    #[command]
    pub fn open_case(&mut self, case_id: String, cx: &mut Context<Self>) {
        if case_id.starts_with("group.") {
            return;
        }
        if let Some(manager) = self.manager.clone() {
            let uri: Uri = format!("rml://{}", case_id).parse().unwrap();
            manager.open(&uri);
            self.sync_tab_state(&manager);
            cx.notify();
        }
    }

    /// 由 LspExplorerPanel::on_file_activate 调用（经 MainWindowRef 回调）。
    #[command]
    pub fn open_lsp_file(&mut self, relative_path: String, cx: &mut Context<Self>) {
        if let Some(manager) = self.manager.clone() {
            let uri: Uri = format!("lsp://{}", relative_path).parse().unwrap();
            manager.open(&uri);
            self.sync_tab_state(&manager);
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
        // 刷新 menus（t_static 自动读取新 locale）+ status（名称经贡献 name() 刷新）
        self.menus = self.build_menu_tree();
        self.status = {
            let entries = self.entries.read().unwrap();
            build_status_view_models(&entries)
        };
        cx.notify();
    }
}

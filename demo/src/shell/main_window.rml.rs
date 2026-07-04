use std::sync::{Arc, RwLock};

use gpui::{IntoElement, WeakEntity, Window};
use rml::prelude::*;
use rml_app::IAppContextExt;
use rml_core::command::{ICommand, RelayCommand};
use rml_core::contribution::{ContributionOptions, IContribution, IContributionHost, VisualAbilityExt};
use rml_core::i18n::{t_static, I18nExt};
use rml_core::theme::ThemeExt;
use rml_core::workbench::{IWorkbench, IWorkbenchManager, Uri};
use rml_ui::{ActivityBar, IActivityPanel, VisualActivityPanel};

use crate::lsp::LspClient;
use crate::lsp::lsp_explorer_panel::LspExplorerPanel;
use crate::shell::activity_panel::ActivityPanel;
use crate::shell::case_view_model::CaseViewModel;
use crate::shell::menu_view_model::MenuViewModel;
use crate::shell::status_view_model::{build_status_view_models, ContribEntry, StatusViewModel};
use crate::shell::workbench::{register_workbench_abilities, CaseWorkbench, LspWorkbenchProvider};

/// MainWindow 弱引用槽位——经 IAppContext::set_service 注册为单例，
/// ActivityPanel / LspExplorerPanel / 菜单命令通过 get_service::<MainWindowRef>() 查询。
pub struct MainWindowRef(pub WeakEntity<MainWindow>);

/// MainWindow：`demo.shell` host + ViewModel + IWorkbenchManager。
///
/// 持有 `cases` / `menus` / `status` / `activities` 四个类型化集合，
/// 直接绑定模板（tree / menu-bar / status-bar / ActivityBar）。
/// Tab/资源生命周期由 `IWorkbenchManager` trait 直接管理，
/// 状态存储在 `RwLock` 保护的 `workbenches` / `activated` 字段中。
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

    // Tab 状态（workbenches 派生缓存，命令后同步）
    open_tabs: Vec<Arc<dyn IValue>>,
    selected_tab: usize,
    show_chrome: bool,
    slot_left_size: gpui::Pixels,

    // IWorkbenchManager 状态（RwLock 保护，&self 方法可变）
    workbenches: Arc<RwLock<Vec<Arc<dyn IWorkbench>>>>,
    activated: Arc<RwLock<Option<Arc<dyn IWorkbench>>>>,
    lsp_provider: Arc<LspWorkbenchProvider>,

    // 框架仪式
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    entries: Arc<std::sync::RwLock<Vec<ContribEntry>>>,
    lsp_client: Option<Arc<LspClient>>,
}

/// MainWindow 的 host handle —— 直接操作共享的 `Arc<RwLock<Vec<ContribEntry>>>`。
///
/// 在 `on_loaded` 中创建并注册到 `ContributionRegistry`，替代旧的 channel 桥接。
/// `bootstrap_host_contributions` 同步触发 `register → handle.add → entries.push`，
/// 无需 drain。
struct MainWindowHostHandle {
    id: &'static str,
    entries: Arc<std::sync::RwLock<Vec<ContribEntry>>>,
}

impl IContributionHost for MainWindowHostHandle {
    fn id(&self) -> &'static str {
        self.id
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
            workbenches: Arc::new(RwLock::new(Vec::new())),
            activated: Arc::new(RwLock::new(None)),
            lsp_provider: Arc::new(LspWorkbenchProvider::new(None)),
            activity_bar: None,
            entries: Arc::new(std::sync::RwLock::new(Vec::new())),
            lsp_client: None,
            // #[window] 注入的单一状态字段（替代旧 25+ 个 __rml_* 仪式字段）
            __rml_state: Default::default(),
        }
    }
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.init_contribution_host(cx);
        self.init_commands(cx);
        self.project_entries();
        self.init_services(cx);
        self.init_lsp();
        self.init_workbench(cx);
        self.init_activity_bar(cx);
        self.init_panel_observers(cx);
        cx.notify();
    }
}

impl MainWindow {
    /// 注册 host handle 到 registry + 触发该 host_id 的所有 `#[contribute]` 批量注册。
    fn init_contribution_host(&mut self, cx: &mut Context<Self>) {
        let handle = Arc::new(MainWindowHostHandle {
            id: Self::ID,
            entries: self.entries.clone(),
        });
        cx.get_contribution_registry().add(handle);
        rml_app::contribution::bootstrap_host_contributions(cx, Self::ID);
    }

    /// 初始化 7 个 RelayCommand 字段（WPF MVVM 模式）+ 注册 StatusReady 视觉能力。
    fn init_commands(&mut self, cx: &mut Context<Self>) {
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

        // project_entries 前完成，使 as_visual() 查询生效
        crate::cases::status_bar_case::ensure_status_ready_registered();
    }

    /// 注册 MainWindowRef 单例（ActivityPanel/LspExplorerPanel 经 IAppContext 查询）。
    fn init_services(&mut self, cx: &mut Context<Self>) {
        let shell_weak = cx.weak_entity();
        cx.set_service(Arc::new(MainWindowRef(shell_weak)));
    }

    /// 启动 LSP 子进程（失败时优雅降级）。
    fn init_lsp(&mut self) {
        if let Ok(workspace_root) = std::env::current_dir() {
            match LspClient::spawn(&workspace_root) {
                Ok(client) => self.lsp_client = Some(Arc::new(client)),
                Err(e) => log::warn!("Failed to start LSP server: {e}"),
            }
        }
    }

    /// 初始化 workbench 状态：注册能力 + 构造 LSP provider + 打开 welcome tab。
    fn init_workbench(&mut self, _cx: &mut Context<Self>) {
        register_workbench_abilities();
        self.lsp_provider = Arc::new(LspWorkbenchProvider::new(self.lsp_client.clone()));

        let uri: Uri = "rml://welcome".parse().unwrap();
        if IWorkbenchManager::open(self, &uri).is_some() {
            self.sync_tab_state();
        }
    }

    /// 构建 ActivityBar + 激活首项 + observe active_id 同步 slot_left_size。
    fn init_activity_bar(&mut self, cx: &mut Context<Self>) {
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(self.activities.clone())));

        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.activate_first(cx));
        }

        self.show_chrome = true;
        self.slot_left_size = gpui::px(260.);

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

    /// observe 框架缓存的 ActivityPanel / LspExplorerPanel Entity → 触发重渲。
    fn init_panel_observers(&mut self, cx: &mut Context<Self>) {
        let panel_entity = rml_app::contribution::visual_entity::<ActivityPanel>(cx);
        cx.observe(&panel_entity, |_, _, cx| cx.notify()).detach();

        let lsp_panel_entity = rml_app::contribution::visual_entity::<LspExplorerPanel>(cx);
        cx.observe(&lsp_panel_entity, |_, _, cx| cx.notify()).detach();
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
        let mut activity_entries: Vec<_> = entries
            .iter()
            .filter(|(c, o)| o.effective_slot() == Some("activity") && c.as_visual().is_some())
            .collect();
        activity_entries.sort_by_key(|(_, o)| o.order);
        self.activities = activity_entries
            .into_iter()
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
            MenuViewModel::root(t_static("menu.file"))
                .child(MenuViewModel::leaf(
                    t_static("menu.file_new"),
                    self.open_welcome_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    t_static("menu.file_open"),
                    self.open_button_case_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    t_static("menu.file_exit"),
                    self.exit_command.clone(),
                )),
            MenuViewModel::root(t_static("menu.view"))
                .child(MenuViewModel::leaf(
                    t_static("menu.theme_toggle"),
                    self.toggle_theme_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    t_static("menu.lang_en"),
                    self.switch_en_command.clone(),
                )),
            MenuViewModel::root(t_static("menu.help"))
                .child(
                    MenuViewModel::root(t_static("case.menu.help_center"))
                    .child(MenuViewModel::leaf(
                        t_static("case.menu.nested"),
                        self.open_menu_dropdown_case_command.clone(),
                    ))
                    .child(MenuViewModel::leaf(
                        t_static("menu.help_about"),
                        self.open_welcome_command.clone(),
                    )),
                )
                .child(
                    MenuViewModel::root(t_static("case.menu.features.group"))
                    .child(MenuViewModel::leaf(
                        t_static("case.menu.features.title"),
                        self.open_features_case_command.clone(),
                    )),
                ),
        ]
    }

    /// 同步 tab 状态：从 workbenches/activated 派生 open_tabs + selected_tab。
    fn sync_tab_state(&mut self) {
        let activated_uri = self
            .activated
            .read()
            .unwrap()
            .as_ref()
            .map(|a| a.uri().to_string());
        let workbenches = self.workbenches.read().unwrap();
        self.open_tabs = workbenches
            .iter()
            .map(|w| {
                let iv: Arc<dyn IContribution> = w.clone();
                iv as Arc<dyn IValue>
            })
            .collect();
        self.selected_tab = activated_uri
            .as_ref()
            .and_then(|uri| workbenches.iter().position(|w| w.uri() == uri))
            .unwrap_or(0);
    }

    /// 渲染激活的 workbench 视图：读 activated → as_visual() → render。
    pub fn active_view(&self, window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let activated = self.activated.read().unwrap().clone();
        if let Some(wb) = activated {
            let iv: &dyn IContribution = wb.as_ref();
            if let Some(visual) = iv.as_visual() {
                return visual.render(window, cx);
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
        use gpui_component::menu::DropdownMenu as _;
        use gpui::ParentElement;

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
        self.activate_by_index(index);
        self.sync_tab_state();
        cx.notify();
    }

    /// 由 ActivityPanel::on_case_activate 调用（经 MainWindowRef 回调）。
    #[command]
    pub fn open_case(&mut self, case_id: String, cx: &mut Context<Self>) {
        if case_id.starts_with("group.") {
            return;
        }
        let uri: Uri = format!("rml://{}", case_id).parse().unwrap();
        if IWorkbenchManager::open(self, &uri).is_some() {
            self.sync_tab_state();
            cx.notify();
        }
    }

    /// 由 LspExplorerPanel::on_file_activate 调用（经 MainWindowRef 回调）。
    #[command]
    pub fn open_lsp_file(&mut self, relative_path: String, cx: &mut Context<Self>) {
        let uri: Uri = format!("lsp://{}", relative_path).parse().unwrap();
        if IWorkbenchManager::open(self, &uri).is_some() {
            self.sync_tab_state();
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

// ──────────────────────────────────────────────────────────────────────────
//  IWorkbenchManager 实现：MainWindow 直接管理 Tab/资源生命周期
//
//  点击案例树 → open_case → IWorkbenchManager::open → new Tab → TabWindow 渲染。
//  状态存储在 RwLock 保护的 workbenches/activated 字段中，&self 方法可变。
// ──────────────────────────────────────────────────────────────────────────

impl MainWindow {
    /// 按 URI schema 路由构造 workbench。无法识别的 schema 或找不到的 case 返回 None。
    fn build_workbench(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        match uri.scheme() {
            "rml" => {
                let case_id = uri.as_str().strip_prefix("rml://").unwrap_or("");
                let case = self
                    .cases
                    .iter()
                    .find(|c| c.id == case_id)
                    .cloned()?;
                Some(Arc::new(CaseWorkbench::new(uri.as_str().into(), case)))
            }
            "lsp" => Some(self.lsp_provider.build_workbench(uri)),
            _ => None,
        }
    }

    /// 按 index 激活 workbench（供 on_tab_click 调用）。
    fn activate_by_index(&self, index: usize) {
        let workbenches = self.workbenches.read().unwrap();
        if let Some(wb) = workbenches.get(index) {
            *self.activated.write().unwrap() = Some(wb.clone());
        }
    }
}

impl IWorkbenchManager for MainWindow {
    fn open(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();
        // 去重：已打开则直接激活
        if let Some(wb) = self
            .workbenches
            .read()
            .unwrap()
            .iter()
            .find(|w| w.uri() == uri_str)
            .cloned()
        {
            *self.activated.write().unwrap() = Some(wb.clone());
            return Some(wb);
        }
        let wb = self.build_workbench(uri)?;
        self.workbenches.write().unwrap().push(wb.clone());
        *self.activated.write().unwrap() = Some(wb.clone());
        Some(wb)
    }

    fn close(&self, uri: &Uri) {
        let uri_str = uri.as_str();
        let mut workbenches = self.workbenches.write().unwrap();
        workbenches.retain(|w| w.uri() != uri_str);
        let mut activated = self.activated.write().unwrap();
        if activated.as_ref().map(|a| a.uri() == uri_str).unwrap_or(false) {
            *activated = workbenches.first().cloned();
        }
    }

    fn get_all(&self) -> Vec<Arc<dyn IWorkbench>> {
        self.workbenches.read().unwrap().clone()
    }

    fn get_activated(&self) -> Option<Arc<dyn IWorkbench>> {
        self.activated.read().unwrap().clone()
    }

    fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();
        self.workbenches
            .read()
            .unwrap()
            .iter()
            .find(|w| w.uri() == uri_str)
            .cloned()
    }
}

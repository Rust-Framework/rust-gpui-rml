use std::sync::{Arc, RwLock};

use crossbeam_channel::Receiver;
use gpui::{WeakEntity, Window};
use rml::prelude::*;
use rust_rml_client::{LanguageClient, ServerStatus};
use rml_app::IAppContextExt;
use rml_core::contribution::VisualAbilityExt;
use rml_core::i18n::{I18nExt, I18nState};
use rml_core::observable::ObservableVec;
use rml_core::theme::ThemeExt;
use rml_core::workbench::{IWorkbench, IWorkbenchManager, Uri};
use rml_ui::{ActivityBar, IActivityPanel, VisualActivityPanel};

use crate::lsp::lsp_explorer_panel::LspExplorerPanel;
use crate::lsp::{ensure_lsp_status_item_registered, LspStatusState, LspStatusStateRef};
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
///
/// Tab/资源生命周期由 `IWorkbenchManager` trait 直接管理：
/// - `workbenches: ObservableVec<Arc<dyn IWorkbench>>` — push 时自动 bump 版本 + flume send
/// - `activated: Arc<RwLock<Option<...>>>` — 手动 `__rml_bump_version("activated")` + `cx.notify`
/// - `#[computed] selected_tab` — 依赖 workbenches + activated，版本变化时自动重算
/// - 模板 `<template slot="tabs" each={w in workbenches}>` — 声明式 TabBar 迭代
///
/// 通道桥接：`on_loaded` 中 `flume::unbounded()` + `cx.spawn` 背景任务，
/// ObservableVec 写操作 → `cx.notify()` → GPUI 重渲 → computed 失效 → 模板重新迭代。
///
/// 菜单/状态栏经贡献系统注册（`menu_commands.rs` 声明式定义），
/// `observe_global::<I18nState>` 自动重建 menus/status（响应式重投影）。
#[window]
#[contributehost(id = "demo.shell")]
pub struct MainWindow {
    // 直接绑定模板的集合（on_loaded 后一次性填充）
    pub cases: Vec<CaseViewModel>,
    pub menus: Vec<MenuViewModel>,
    pub status: Vec<StatusViewModel>,
    activities: Vec<Arc<dyn IActivityPanel>>,

    // Tab 状态
    show_chrome: bool,
    slot_left_size: gpui::Pixels,

    // IWorkbenchManager 状态
    // workbenches: ObservableVec 提供版本追踪 + 通道通知，push 时自动 bump 版本 + send()
    // activated: 单值，手动 __rml_bump_version + cx.notify
    workbenches: ObservableVec<Arc<dyn IWorkbench>>,
    activated: Arc<RwLock<Option<Arc<dyn IWorkbench>>>>,
    lsp_provider: Arc<LspWorkbenchProvider>,

    // 框架仪式
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    entries: Arc<std::sync::RwLock<Vec<ContribEntry>>>,
    language_client: Option<Arc<LanguageClient>>,
}

/// 手写 `Default`——`#[window]` 宏注入的版本计数器 / 缓存 / 状态字段全部用 `Default::default()` 初始化。
impl Default for MainWindow {
    fn default() -> Self {
        Self {
            cases: Vec::new(),
            menus: Vec::new(),
            status: Vec::new(),
            activities: Vec::new(),
            show_chrome: false,
            slot_left_size: gpui::px(260.),
            workbenches: ObservableVec::new(),
            activated: Arc::new(RwLock::new(None)),
            lsp_provider: Arc::new(LspWorkbenchProvider::new(None)),
            activity_bar: None,
            entries: Arc::new(std::sync::RwLock::new(Vec::new())),
            language_client: None,
            __rml_state: Default::default(),
        }
    }
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 通道桥接：ObservableVec::push → flume send → 背景任务 cx.notify → 自动 UI 刷新
        let (tx, rx) = flume::unbounded();
        self.workbenches = ObservableVec::with_notify(tx);
        cx.spawn(|this: WeakEntity<MainWindow>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                while rx.recv_async().await.is_ok() {
                    let _ = this.update(&mut cx, |_, cx| cx.notify());
                }
            }
        })
        .detach();

        self.init_contribution_host(cx);
        self.project_entries();
        self.init_services(cx);
        self.init_lsp(cx);
        self.init_workbench(cx);
        self.init_activity_bar(cx);
        self.init_panel_observers(cx);
        self.init_i18n_observer(cx);
    }
}

impl MainWindow {
    /// 注册 host 到 registry + 触发该 host_id 的所有 `#[contribute]` 批量注册。
    /// `register_host` 存 `Arc<dyn IContributionHost>`（`entries.clone()` 经 unsized coercion 转入）；
    /// `bootstrap` 同步触发 `register → host.add(c, opts) → storage.write().push`。
    /// `ensure_status_ready_registered` 在 bootstrap 后调用，使 `as_visual()` 查询生效。
    fn init_contribution_host(&mut self, cx: &mut Context<Self>) {
        cx.register_host(Self::ID, self.entries.clone());
        rml_app::contribution::bootstrap_host_contributions(cx, Self::ID);
        crate::cases::status_bar_case::ensure_status_ready_registered();
        ensure_lsp_status_item_registered();
    }

    /// 注册 MainWindowRef + LspStatusStateRef 单例（经 IAppContext 查询）。
    fn init_services(&mut self, cx: &mut Context<Self>) {
        let shell_weak = cx.weak_entity();
        cx.set_service(Arc::new(MainWindowRef(shell_weak)));

        let lsp_status = cx.new(|_| LspStatusState::new());
        cx.set_service(Arc::new(LspStatusStateRef(lsp_status.downgrade())));
    }

    /// 启动语言服务子进程（失败时优雅降级）+ 订阅 RA 加载状态。
    fn init_lsp(&mut self, cx: &mut Context<Self>) {
        if let Ok(workspace_root) = std::env::current_dir() {
            match LanguageClient::unified(&workspace_root) {
                Ok(client) => {
                    let rx = client.status_receiver();
                    self.language_client = Some(Arc::new(client));
                    self.spawn_status_listener(rx, cx);
                }
                Err(e) => log::warn!("Failed to start language server: {e}"),
            }
        }
    }

    /// 后台轮询 `rml/serverStatus` 通知，更新 `LspStatusState` Entity。
    ///
    /// crossbeam `Receiver::recv()` 阻塞调用，不能在 foreground executor 直接调用。
    /// 采用 `cx.spawn` + `cx.background_executor().spawn()` 嵌套：foreground 循环
    /// await background 的单次 recv，既不阻塞 UI 又能持续轮询。
    fn spawn_status_listener(&self, rx: Receiver<ServerStatus>, cx: &mut Context<Self>) {
        let Some(lsp_status_ref) = cx.get_service::<LspStatusStateRef>() else {
            return;
        };
        let weak = lsp_status_ref.0.clone();

        cx.spawn(move |_this, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    let rx = rx.clone();
                    let result = cx
                        .background_executor()
                        .spawn(async move { rx.recv() })
                        .await;
                    match result {
                        Ok(status) => {
                            if let Some(entity) = weak.upgrade() {
                                let _ = entity.update(&mut cx, |this, cx| {
                                    this.set_server_status(status, cx);
                                });
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        })
        .detach();
    }

    /// 初始化 workbench 状态：注册能力 + 构造 LSP provider + 打开 welcome tab。
    fn init_workbench(&mut self, _cx: &mut Context<Self>) {
        register_workbench_abilities();
        self.lsp_provider = Arc::new(LspWorkbenchProvider::new(self.language_client.clone()));

        let uri: Uri = "rml://welcome".parse().unwrap();
        if IWorkbenchManager::open(self, &uri).is_some() {
            self.__rml_bump_version("activated");
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

    /// observe 框架缓存的 ActivityPanel / LspExplorerPanel Entity + LspStatusState → 触发重渲。
    fn init_panel_observers(&mut self, cx: &mut Context<Self>) {
        let panel_entity = rml_app::contribution::visual_entity::<ActivityPanel>(cx);
        cx.observe(&panel_entity, |_, _, cx| {
            cx.notify();
        }).detach();

        let lsp_panel_entity = rml_app::contribution::visual_entity::<LspExplorerPanel>(cx);
        cx.observe(&lsp_panel_entity, |_, _, cx| {
            cx.notify();
        }).detach();

        // LspStatusState 变化 → 状态栏重渲（LspStatusItem::render 读取最新消息）
        if let Some(entity) = cx
            .get_service::<LspStatusStateRef>()
            .and_then(|r| r.0.upgrade())
        {
            cx.observe(&entity, |_, _, cx| {
                cx.notify();
            })
            .detach();
        }
    }

    /// observe `I18nState` 全局变化 → 自动重建 menus/status ViewModel + 重渲。
    /// locale 切换时 `cx.set_i18n` 内部触发 `update_global::<I18nState>`，
    /// 此 observer 回调重投影 menus/status（标签来自 `contribution.name()` 反映新 locale）。
    fn init_i18n_observer(&mut self, cx: &mut Context<Self>) {
        cx.observe_global::<I18nState>(|this, cx| {
            this.rebuild_i18n_dependent();
            cx.notify();
        })
        .detach();
    }
}

impl MainWindow {
    /// 从 entries 暂存投影到 cases/menus/status/activities 类型化集合。
    fn project_entries(&mut self) {
        let entries = self.entries.read().unwrap();
        self.cases = entries
            .iter()
            .filter_map(|(c, o)| CaseViewModel::from_contribution(c.clone(), o.clone()))
            .collect();
        self.menus = MenuViewModel::build_menu_view_models(&entries);
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
    }

    /// 重建 i18n 依赖的 ViewModel 集合（menus + status）。
    /// 由 `observe_global::<I18nState>` 在 locale 变化时自动调用。
    fn rebuild_i18n_dependent(&mut self) {
        let entries = self.entries.read().unwrap();
        self.menus = MenuViewModel::build_menu_view_models(&entries);
        self.status = build_status_view_models(&entries);
    }

    /// 当前激活的 Tab 索引（#[computed] 自动缓存，依赖 workbenches + activated 版本）。
    /// workbenches 版本由 ObservableVec::push 内部 fetch_add 自动递增；
    /// activated 版本由命令方法手动 __rml_bump_version("activated") 递增。
    #[computed]
    pub fn selected_tab(&self) -> usize {
        let activated_uri = self
            .activated
            .read()
            .unwrap()
            .as_ref()
            .map(|a| a.uri().to_string());
        let workbenches = self.workbenches.snapshot();
        activated_uri
            .as_ref()
            .and_then(|uri| workbenches.iter().position(|w| w.uri() == uri))
            .unwrap_or(0)
    }

    /// Tab 项列表（#[computed] 自动缓存，依赖 workbenches 版本）。
    /// 将 `ObservableVec<Arc<dyn IWorkbench>>` 转换为 `Vec<Arc<dyn IValue>>`，
    /// 供 `<tab-window tabs={tab_items}>` 简单绑定模式使用。
    #[computed]
    pub fn tab_items(&self) -> Vec<Arc<dyn rml_core::value::IValue>> {
        self.workbenches
            .snapshot()
            .into_iter()
            .map(|w| w as Arc<dyn rml_core::value::IValue>)
            .collect()
    }

    /// 状态栏左侧项（#[computed] 自动缓存，依赖 status 版本）。
    #[computed]
    pub fn status_left(&self) -> Vec<StatusViewModel> {
        self.status
            .iter()
            .filter(|s| s.align == rml_ui::StatusBarAlign::Left)
            .cloned()
            .collect()
    }

    /// 状态栏居中项（#[computed] 自动缓存，依赖 status 版本）。
    #[computed]
    pub fn status_center(&self) -> Vec<StatusViewModel> {
        self.status
            .iter()
            .filter(|s| s.align == rml_ui::StatusBarAlign::Center)
            .cloned()
            .collect()
    }

    /// 状态栏右侧项（#[computed] 自动缓存，依赖 status 版本）。
    #[computed]
    pub fn status_right(&self) -> Vec<StatusViewModel> {
        self.status
            .iter()
            .filter(|s| s.align == rml_ui::StatusBarAlign::Right)
            .cloned()
            .collect()
    }

    #[command]
    pub fn on_chrome_toggle(&mut self, cx: &mut Context<Self>) {
        self.show_chrome = !self.show_chrome;
    }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
        self.activate_by_index(index);
        self.__rml_bump_version("activated");
    }

    /// 关闭指定索引的 tab：调用 `IWorkbenchManager::close` 移除 workbench，
    /// 若是当前激活项则切到首个剩余项；手动 bump `activated` 版本（close 仅 bump
    /// `workbenches`）以触发 `selected_tab` computed 失效与 RML 重投影。
    #[command]
    pub fn on_tab_close(&mut self, index: usize, cx: &mut Context<Self>) {
        let wb = self.workbenches.snapshot().get(index).cloned();
        if let Some(wb) = wb {
            let uri: Uri = wb.uri().parse().unwrap();
            IWorkbenchManager::close(self, &uri);
            self.__rml_bump_version("activated");
        }
    }

    /// 关闭全部 workbench：清空后激活项置 None，bump `activated` 触发
    /// `selected_tab` computed 失效与 RML 重投影。
    #[command]
    pub fn on_tab_close_all(&mut self, cx: &mut Context<Self>) {
        if self.workbenches.is_empty() {
            return;
        }
        self.workbenches.clear();
        *self.activated.write().unwrap() = None;
        self.__rml_bump_version("activated");
    }

    /// 关闭其他 workbench：仅保留 index 对应项。clear + 重 push 保留项，
    /// 避免 `remove_where` 仅移除首个导致的循环；activated 切到保留项。
    #[command]
    pub fn on_tab_close_others(&mut self, index: usize, cx: &mut Context<Self>) {
        let keep = match self.workbenches.snapshot().get(index).cloned() {
            Some(wb) => wb,
            None => return,
        };
        self.workbenches.clear();
        self.workbenches.push(keep.clone());
        *self.activated.write().unwrap() = Some(keep);
        self.__rml_bump_version("activated");
    }

    /// 由 ActivityPanel::on_case_activate 调用（经 MainWindowRef 回调）。
    #[command]
    pub fn open_case(&mut self, case_id: String, cx: &mut Context<Self>) {
        if case_id.starts_with("group.") {
            return;
        }
        let uri: Uri = format!("rml://{}", case_id).parse().unwrap();
        if IWorkbenchManager::open(self, &uri).is_some() {
            self.__rml_bump_version("activated");
        }
    }

    /// 由 LspExplorerPanel::on_file_activate 调用（经 MainWindowRef 回调）。
    #[command]
    pub fn open_lsp_file(&mut self, relative_path: String, cx: &mut Context<Self>) {
        let uri: Uri = format!("lsp://{}", relative_path).parse().unwrap();
        if IWorkbenchManager::open(self, &uri).is_some() {
            self.__rml_bump_version("activated");
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
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  IWorkbenchManager 实现：MainWindow 直接管理 Tab/资源生命周期
//
//  点击案例树 → open_case → IWorkbenchManager::open → workbenches.push(wb) →
//  ObservableVec 版本递增 + flume 通道通知 → 背景任务 cx.notify →
//  #[computed] selected_tab 失效重算 → 模板 each={w in workbenches} 重新迭代 →
//  TabBar 自动新增 Tab 并激活。零手动同步代码。
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
        if let Some(wb) = self.workbenches.get(index) {
            *self.activated.write().unwrap() = Some(wb);
        }
    }
}

impl IWorkbenchManager for MainWindow {
    fn open(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();
        // 去重：已打开则直接激活
        if let Some(wb) = self.workbenches.snapshot().into_iter().find(|w| w.uri() == uri_str) {
            *self.activated.write().unwrap() = Some(wb.clone());
            return Some(wb);
        }
        let wb = self.build_workbench(uri)?;
        // push 触发 ObservableVec 内部 fetch_add + flume send → 背景任务 cx.notify
        self.workbenches.push(wb.clone());
        *self.activated.write().unwrap() = Some(wb.clone());
        Some(wb)
    }

    fn close(&self, uri: &Uri) {
        let uri_str = uri.as_str();
        let closed_index = self
            .workbenches
            .snapshot()
            .iter()
            .position(|w| w.uri() == uri_str);
        self.workbenches.remove_where(|w| w.uri() == uri_str);
        let mut activated = self.activated.write().unwrap();
        if activated.as_ref().map(|a| a.uri() == uri_str).unwrap_or(false) {
            let new_snapshot = self.workbenches.snapshot();
            // 就近左侧激活：关闭 index N → 激活 N-1；关闭首项 → 激活新首项；
            // 无剩余 → None。匹配浏览器 Tab 关闭交互。
            *activated = closed_index.and_then(|ix| {
                if new_snapshot.is_empty() {
                    None
                } else {
                    new_snapshot.get(ix.saturating_sub(1)).cloned()
                }
            });
        }
    }

    fn get_all(&self) -> Vec<Arc<dyn IWorkbench>> {
        self.workbenches.snapshot()
    }

    fn get_activated(&self) -> Option<Arc<dyn IWorkbench>> {
        self.activated.read().unwrap().clone()
    }

    fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();
        self.workbenches
            .snapshot()
            .into_iter()
            .find(|w| w.uri() == uri_str)
    }
}

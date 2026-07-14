//! MainWindow —— Arc Studio 主窗口(`#[window]` 声明式 GPUI 窗口)。
//!
//! 持有 `Arc<ArcShellManager>`(DI singleton)用于 Tab/资源生命周期管理。
//! `workbenches` / `activated` 与 manager 共享底层数据(RwLock + ObservableVec 的 Arc 共享),
//! `#[computed]` 经版本号追踪变更,无需镜像同步。
//!
//! # 贡献宿主
//!
//! `#[contributehost(id = "studio.shell")]` 声明 MainWindow 为贡献宿主,
//! 菜单/状态栏项经 `#[contribute(host_id = "studio.shell")]` 声明式注册。
//! `on_loaded` 中 `register_host` + `bootstrap_host_contributions` 触发批量注册,
//! `project_entries` 将共享存储投影到 `menus`/`status` 类型化集合供模板绑定。
//!
//! # 通道桥接
//!
//! `on_loaded` 中 `cx.spawn` 背景任务:
//! manager 写操作(push/remove) → flume send → 背景任务 `cx.notify()` + `__rml_bump_version("activated")`
//! → GPUI 重渲 → `#[computed]` 版本变化重算 → 模板重新迭代。

use std::sync::{Arc, RwLock};

use rml::prelude::*;
use rml_app::IAppContextExt;
use rml_core::i18n::{I18nExt, I18nState};
use rml_core::observable::ObservableVec;
use rml_core::theme::ThemeExt;
use rml_core::value::IValue;
use rml_core::workbench::{IWorkbench, IWorkbenchManager, Uri};
use rml_ui::{
    ActivityBar, IActivityPanel, StatusBarAlign, VisualActivityPanel, get_activity_panels,
};
use studio_core::{open_workspace, workspace::IWorkspaceManager};

use crate::di;
use crate::menu_view_model::MenuViewModel;
use crate::shell_manager::ArcShellManager;
use crate::status_items::ensure_status_ready_registered;
use crate::status_view_model::{ContribEntry, StatusViewModel, build_status_view_models};

/// Arc Studio 主窗口 —— `#[window]` 声明式 GPUI 窗口 + `#[contributehost]` 贡献宿主。
///
/// - `manager` —— DI singleton,impl `IWorkbenchManager` + `IWorkspaceManager`
/// - `workbenches` —— 与 manager 共享的 `ObservableVec` 副本(版本号 + 数据共享)
/// - `activated` —— 与 manager 共享的 `Arc<RwLock<...>>`(同一 RwLock 实例)
/// - `entries` —— 贡献宿主共享存储,`register_host` 注册后 `#[contribute]` 批量写入
/// - `menus` / `status` —— 从 `entries` 投影的类型化集合,直接绑定模板
/// - `activity_bar` —— 左侧 ActivityBar(GPUI Entity),`on_loaded` 中手动创建
/// - `activities` —— ActivityBar 的面板列表(经注册表发现)
///
/// `#[computed] tab_items` / `selected_tab` 依赖上述字段的版本号,
/// manager 写操作经 flume 通道触发背景任务 `cx.notify()` → 重渲 → computed 失效重算。
#[window]
#[contributehost(id = "studio.shell")]
pub struct MainWindow {
    manager: Arc<ArcShellManager>,
    /// 与 manager.workbenches 共享底层数据 + 版本号的副本。
    /// `#[computed] tab_items` 经 `self.workbenches.version()` 追踪变更。
    workbenches: ObservableVec<Arc<dyn IWorkbench>>,
    /// 与 manager.activated 共享同一 RwLock 实例。
    /// `on_tab_click` 直接写入,`#[computed] selected_tab` 直接读取 —— 无需镜像同步。
    activated: Arc<RwLock<Option<Arc<dyn IWorkbench>>>>,
    /// 贡献宿主共享存储 —— `register_host` 注册后,`#[contribute]` 批量写入。
    /// `project_entries` 从此存储投影到 `menus`/`status` 类型化集合。
    entries: Arc<RwLock<Vec<ContribEntry>>>,
    /// 菜单树(从 `entries` 投影,`each` 指令要求字段而非方法)。
    menus: Vec<MenuViewModel>,
    /// 状态栏项列表(从 `entries` 投影,经 `#[computed] status_left/center/right` 分区)。
    status: Vec<StatusViewModel>,
    show_chrome: bool,
    /// 左侧 ActivityBar 插槽宽度(折叠时 48px,展开时 >60px 触发展开)。
    /// 实际拖拽宽度由 TabWindowShell 经 keyed_state 持久化,此字段仅控制折叠/展开状态。
    slot_left_size: gpui::Pixels,
    /// ActivityBar Entity —— `on_loaded` 中手动创建,经 `ref="activity_bar"` 绑定模板。
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    /// ActivityBar 面板列表(经 `get_activity_panels` 注册表发现)。
    activities: Vec<Arc<dyn IActivityPanel>>,
}

impl Default for MainWindow {
    fn default() -> Self {
        // Default 阶段创建 manager(无 provider,仅空集合 + 通道)。
        // on_loaded 时经 di::build_runtime_provider(manager) 构建 DI 容器并反向注入 provider。
        let manager = Arc::new(ArcShellManager::new());
        let workbenches = manager.workbenches_handle();
        let activated = manager.activated_handle();
        Self {
            manager,
            workbenches,
            activated,
            entries: Arc::new(RwLock::new(Vec::new())),
            menus: Vec::new(),
            status: Vec::new(),
            show_chrome: true,
            slot_left_size: gpui::px(260.),
            activity_bar: None,
            activities: Vec::new(),
            __rml_state: Default::default(),
        }
    }
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 0. 初始化 i18n + 主题(必须在贡献注册之前,确保 t_static 能读到翻译)
        cx.use_i18n_with_dir("zh-CN", "assets/i18n");
        cx.use_theme_with_dir("dark", "assets/themes");

        // 1. 构建 DI 容器(注册 manager + apply_auto_registrations,反向注入 provider 到 manager)
        let provider = di::build_runtime_provider(self.manager.clone());

        // 2. 追加 provider 到 provider 链（configure 阶段的静态服务仍可解析）
        //    业务代码经 cx.get_trait::<dyn T>() 解析（经 ServiceProviderExt）
        cx.use_provider(provider);

        // 3. 启动通道桥接背景任务:flume recv → cx.notify + bump activated 版本
        //    manager 的 push/remove/close 经 ObservableVec::bump → flume send → 此任务唤醒
        let rx = self.manager.notify_receiver();
        cx.spawn(|this: WeakEntity<MainWindow>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                while rx.recv_async().await.is_ok() {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.__rml_bump_version("activated");
                        cx.notify();
                    });
                }
            }
        })
        .detach();

        // 4. 注册贡献宿主 + 触发 #[contribute] 批量注册 + 注册 StatusReady IVisual 能力
        self.init_contribution_host(cx);

        // 5. 从 entries 投影到 menus/status/activities 类型化集合
        self.project_entries();

        // 6. 注册 MainWindowRef 单例(菜单命令经此查询 MainWindow entity)
        self.init_services(cx);

        // 7. 初始化默认工作空间(经注册表尝试以当前目录打开工作空间)
        self.init_default_workspace();

        // 8. 初始化 ActivityBar(经注册表发现面板 → VisualActivityPanel 适配 → ActivityBar Entity)
        //    必须在 init_default_workspace 之后 —— 面板 on_loaded 会查询 IWorkspaceManager
        //    observer 仅切换 slot_left_size 控制折叠/展开,实际宽度持久化由 TabWindowShell 处理
        self.init_activity_bar(cx);

        // 9. observe I18nState 全局变化 → 自动重建 menus/status + 重渲。
        //     locale 切换时 `cx.set_i18n` 内部触发 `update_global::<I18nState>`，
        //     此 observer 回调重投影 menus/status（标签来自 contribution.name() 反映新 locale）。
        cx.observe_global::<I18nState>(|this, cx| {
            this.rebuild_i18n_dependent();
            cx.notify();
        })
        .detach();

        // 10. 打开欢迎页(manager.open → push workbench → ObservableVec bump + flume send
        //     → 背景任务 cx.notify → #[computed] 重算 → TabBar 新增 Tab)
        let uri: Uri = "rml://welcome".parse().unwrap();
        if self.manager.open(&uri).is_some() {
            self.__rml_bump_version("activated");
        }
    }
}

impl MainWindow {
    /// 注册贡献宿主 + 触发 `studio.shell` 的所有 `#[contribute]` 批量注册。
    /// `register_host` 存 `Arc<dyn IContributionHost>`（`entries.clone()` 经 unsized coercion 转入）；
    /// `bootstrap` 同步触发 `register → host.add(c, opts) → storage.write().push`。
    fn init_contribution_host(&mut self, cx: &mut Context<Self>) {
        cx.register_host(Self::ID, self.entries.clone());
        rml_app::contribution::bootstrap_host_contributions(cx, Self::ID);
        ensure_status_ready_registered();
    }

    /// 注册 MainWindowRef 单例（经 IAppContext 查询）。
    /// 菜单命令经 `ctx.app.get_service::<MainWindowRef>()` 查询 MainWindow entity。
    fn init_services(&mut self, cx: &mut Context<Self>) {
        let shell_weak = cx.weak_entity();
        cx.set_service(Arc::new(MainWindowRef(shell_weak)));
    }

    /// 从 entries 暂存投影到 menus/status 类型化集合。
    fn project_entries(&mut self) {
        let entries = self.entries.read().unwrap();
        self.menus = MenuViewModel::build_menu_view_models(&entries);
        self.status = build_status_view_models(&entries);
    }

    /// 重建 i18n 依赖的 ViewModel 集合（menus + status）。
    /// 由 `observe_global::<I18nState>` 在 locale 变化时自动调用。
    fn rebuild_i18n_dependent(&mut self) {
        let entries = self.entries.read().unwrap();
        self.menus = MenuViewModel::build_menu_view_models(&entries);
        self.status = build_status_view_models(&entries);
    }

    /// 初始化默认工作空间 —— 经注册表尝试以当前目录打开工作空间。
    ///
    /// 无已注册 opener 能处理时静默跳过(用户可后续经命令打开其他目录)。
    fn init_default_workspace(&self) {
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(ws) = open_workspace(&cwd) {
                self.manager.add(ws);
            } else {
                log::info!("MainWindow: current dir is not a recognized workspace");
            }
        }
    }

    /// 构建 ActivityBar + 激活首项 + observe active_id 同步 slot_left_size。
    ///
    /// 枚举已注册面板 → VisualActivityPanel 适配 → ActivityBar Entity。
    /// 面板经各扩展 crate `#[ctor::ctor]` + `register_activity_panel` 自注册,
    /// 此处经 `get_activity_panels()` 枚举并适配为 `IActivityPanel`。
    ///
    /// observer 仅切换 slot_left_size 控制折叠/展开:
    /// - 折叠:slot_left_size = 48px (<= SLOT_COLLAPSED_THRESHOLD)
    /// - 展开:slot_left_size = 260px (> SLOT_COLLAPSED_THRESHOLD)
    /// 实际拖拽宽度的持久化由 TabWindowShell::render 经 keyed_state 处理,
    /// observer 不再操作 h_state。
    fn init_activity_bar(&mut self, cx: &mut Context<Self>) {
        self.activities = get_activity_panels()
            .into_iter()
            .filter_map(|contrib| VisualActivityPanel::new(contrib))
            .map(|p| Arc::new(p) as Arc<dyn IActivityPanel>)
            .collect();

        // 创建 ActivityBar Entity
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(self.activities.clone())));

        // 激活首项(触发首项面板渲染 → on_loaded)
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| {
                bar.activate_first(cx);
            });
        }

        self.show_chrome = true;
        self.slot_left_size = gpui::px(260.);

        // observe ActivityBar active_id 变化 → 切换 slot_left_size 控制折叠/展开
        if let Some(bar) = &self.activity_bar {
            cx.observe(bar, move |this, bar, cx| {
                let collapsed = bar.read(cx).active_id().is_none();
                if collapsed {
                    this.slot_left_size = gpui::px(48.);
                } else {
                    this.slot_left_size = gpui::px(260.);
                }
                cx.notify();
            })
            .detach();
        }
    }
}

impl MainWindow {
    /// Tab 项列表(#[computed] 自动缓存,依赖 workbenches 版本)。
    ///
    /// 将 `ObservableVec<Arc<dyn IWorkbench>>` 转换为 `Vec<Arc<dyn IValue>>`,
    /// 供 `<tab-window tabs={tab_items}>` 简单绑定模式使用。
    #[computed]
    pub fn tab_items(&self) -> Vec<Arc<dyn IValue>> {
        self.workbenches
            .snapshot()
            .into_iter()
            .map(|w| w as Arc<dyn IValue>)
            .collect()
    }

    /// 当前激活的 Tab 索引(#[computed] 自动缓存,依赖 workbenches + activated 版本)。
    ///
    /// workbenches 版本由 ObservableVec::push/remove 内部 fetch_add 递增;
    /// activated 版本由 `on_tab_click` / `on_tab_close` / 背景任务手动 bump。
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

    /// 状态栏左侧项（#[computed] 自动缓存,依赖 status 版本）。
    #[computed]
    pub fn status_left(&self) -> Vec<StatusViewModel> {
        self.status
            .iter()
            .filter(|s| s.align == StatusBarAlign::Left)
            .cloned()
            .collect()
    }

    /// 状态栏居中项（#[computed] 自动缓存,依赖 status 版本）。
    #[computed]
    pub fn status_center(&self) -> Vec<StatusViewModel> {
        self.status
            .iter()
            .filter(|s| s.align == StatusBarAlign::Center)
            .cloned()
            .collect()
    }

    /// 状态栏右侧项（#[computed] 自动缓存,依赖 status 版本）。
    #[computed]
    pub fn status_right(&self) -> Vec<StatusViewModel> {
        self.status
            .iter()
            .filter(|s| s.align == StatusBarAlign::Right)
            .cloned()
            .collect()
    }

    /// 点击 Tab:激活对应工作台。直接写入共享 activated,bump 版本触发 computed 重算。
    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(wb) = self.workbenches.get(index) {
            *self.activated.write().unwrap() = Some(wb);
        }
        self.__rml_bump_version("activated");
    }

    /// 关闭 Tab:经 manager.close 移除工作台(ObservableVec bump + flume send)。
    /// 手动 bump activated(close 仅 bump workbenches)以触发 selected_tab computed 重算。
    #[command]
    pub fn on_tab_close(&mut self, index: usize, cx: &mut Context<Self>) {
        let wb = self.workbenches.snapshot().get(index).cloned();
        if let Some(wb) = wb {
            let uri: Uri = wb.uri().parse().unwrap();
            self.manager.close(&uri);
            self.__rml_bump_version("activated");
        }
    }

    /// 关闭全部 Tab(不可关闭的 Tab 如欢迎页保留)。
    #[command]
    pub fn on_tab_close_all(&mut self, cx: &mut Context<Self>) {
        let uris: Vec<Uri> = self
            .workbenches
            .snapshot()
            .iter()
            .filter(|w| w.closable())
            .filter_map(|w| w.uri().parse().ok())
            .collect();
        for uri in uris {
            self.manager.close(&uri);
        }
        self.__rml_bump_version("activated");
    }

    /// 关闭其他 Tab(保留指定索引的 Tab + 不可关闭的 Tab)。
    #[command]
    pub fn on_tab_close_others(&mut self, index: usize, cx: &mut Context<Self>) {
        let snapshot = self.workbenches.snapshot();
        let keep_uri = snapshot.get(index).map(|w| w.uri().to_string());
        let uris: Vec<Uri> = snapshot
            .iter()
            .filter(|w| w.closable() && Some(w.uri()) != keep_uri.as_deref())
            .filter_map(|w| w.uri().parse().ok())
            .collect();
        for uri in uris {
            self.manager.close(&uri);
        }
        self.__rml_bump_version("activated");
    }

    /// 双击 Tab 触发升级(将 preview Tab 转为正式 Tab)。
    ///
    /// 经 manager.promote 取消预览标记,TabWindowShell 据此重渲 italic → 正常字体。
    /// bump activated 版本触发 selected_tab / tab_items computed 重算。
    #[command]
    pub fn on_tab_promote(&mut self, index: usize, cx: &mut Context<Self>) {
        let snapshot = self.workbenches.snapshot();
        if let Some(wb) = snapshot.get(index) {
            if let Ok(uri) = wb.uri().parse::<Uri>() {
                self.manager.promote(&uri);
                self.__rml_bump_version("activated");
                cx.notify();
            }
        }
    }
}

impl MainWindow {
    /// 切换主题(dark ↔ light)。由菜单命令 `ToggleThemeCommand` 调用。
    pub fn apply_toggle_theme(&mut self, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" {
            "light"
        } else {
            "dark"
        };
        cx.set_theme(next);
        cx.notify();
    }

    /// 切换到英文。由菜单命令 `SwitchEnCommand` 调用。
    pub fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
        cx.set_i18n("en-US");
    }

    /// 切换到中文。由菜单命令 `SwitchZhCommand` 调用。
    pub fn apply_switch_zh(&mut self, cx: &mut Context<Self>) {
        cx.set_i18n("zh-CN");
    }

    /// 打开欢迎页。由菜单命令 `AboutCommand` 调用。
    pub fn open_welcome(&mut self, _cx: &mut Context<Self>) {
        let uri: Uri = "rml://welcome".parse().unwrap();
        if self.manager.open(&uri).is_some() {
            self.__rml_bump_version("activated");
        }
    }
}

/// MainWindow 弱引用槽位——经 `IAppContext::set_service` 注册为单例，
/// 菜单命令通过 `get_service::<MainWindowRef>()` 查询。
pub struct MainWindowRef(pub gpui::WeakEntity<MainWindow>);

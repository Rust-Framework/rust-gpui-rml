//! MainWindow —— Arc Studio 主窗口(`#[window]` 声明式 GPUI 窗口)。
//!
//! 持有 `Arc<ArcShellManager>`(DI singleton)用于 Tab/资源生命周期管理。
//! `workbenches` / `activated` 与 manager 共享底层数据(RwLock + ObservableVec 的 Arc 共享),
//! `#[computed]` 经版本号追踪变更,无需镜像同步。
//!
//! # 通道桥接
//!
//! `on_loaded` 中 `cx.spawn` 背景任务:
//! manager 写操作(push/remove) → flume send → 背景任务 `cx.notify()` + `__rml_bump_version("activated")`
//! → GPUI 重渲 → `#[computed]` 版本变化重算 → 模板重新迭代。

use std::sync::{Arc, RwLock};

use rml::prelude::*;
use rml_core::context::IAppContext;
use rml_core::observable::ObservableVec;
use rml_core::value::IValue;
use rml_core::workbench::{IWorkbench, IWorkbenchManager, Uri};
use rml_ui::{ActivityBar, IActivityPanel, VisualActivityPanel, get_activity_panels};
use studio_core::workspace::{IWorkspace, IWorkspaceManager};
use studio_explorer::git_worktree::GitWorktree;

use crate::di;
use crate::shell_manager::ArcShellManager;

/// Arc Studio 主窗口 —— `#[window]` 声明式 GPUI 窗口。
///
/// - `manager` —— DI singleton,impl `IWorkbenchManager` + `IWorkspaceManager`
/// - `workbenches` —— 与 manager 共享的 `ObservableVec` 副本(版本号 + 数据共享)
/// - `activated` —— 与 manager 共享的 `Arc<RwLock<...>>`(同一 RwLock 实例)
/// - `activity_bar` —— 左侧 ActivityBar(GPUI Entity),`on_loaded` 中手动创建
/// - `activities` —— ActivityBar 的面板列表(ExplorerPanel 等)
///
/// `#[computed] tab_items` / `selected_tab` 依赖上述字段的版本号,
/// manager 写操作经 flume 通道触发背景任务 `cx.notify()` → 重渲 → computed 失效重算。
#[window]
pub struct MainWindow {
    manager: Arc<ArcShellManager>,
    /// 与 manager.workbenches 共享底层数据 + 版本号的副本。
    /// `#[computed] tab_items` 经 `self.workbenches.version()` 追踪变更。
    workbenches: ObservableVec<Arc<dyn IWorkbench>>,
    /// 与 manager.activated 共享同一 RwLock 实例。
    /// `on_tab_click` 直接写入,`#[computed] selected_tab` 直接读取 —— 无需镜像同步。
    activated: Arc<RwLock<Option<Arc<dyn IWorkbench>>>>,
    show_chrome: bool,
    /// 左侧 ActivityBar 插槽宽度(折叠时 48px,展开时 260px)。
    slot_left_size: gpui::Pixels,
    /// ActivityBar Entity —— `on_loaded` 中手动创建,经 `ref="activity_bar"` 绑定模板。
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    /// ActivityBar 面板列表(ExplorerPanel 经 VisualActivityPanel 适配)。
    activities: Vec<Arc<dyn IActivityPanel>>,
}

impl Default for MainWindow {
    fn default() -> Self {
        // Default 阶段创建 manager(无 provider,仅空集合 + 通道)。
        // on_loaded 时经 di::build_provider(manager) 构建 DI 容器并反向注入 provider。
        let manager = Arc::new(ArcShellManager::new());
        let workbenches = manager.workbenches_handle();
        let activated = manager.activated_handle();
        Self {
            manager,
            workbenches,
            activated,
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
        // 1. 构建 DI 容器(注册 manager + WelcomeProvider,反向注入 provider 到 manager)
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

        // 4. 初始化默认工作空间(打开当前目录为 GitWorktree)
        self.init_default_workspace();

        // 5. 初始化 ActivityBar(创建 ExplorerPanel + VisualActivityPanel + ActivityBar)
        //    必须在 init_default_workspace 之后 —— ExplorerPanel::on_loaded 会查询 IWorkspaceManager
        self.init_activity_bar(cx);

        // 6. 打开欢迎页(manager.open → push workbench → ObservableVec bump + flume send
        //    → 背景任务 cx.notify → #[computed] 重算 → TabBar 新增 Tab)
        let uri: Uri = "rml://welcome".parse().unwrap();
        if self.manager.open(&uri).is_some() {
            self.__rml_bump_version("activated");
        }
    }
}

impl MainWindow {
    /// 初始化默认工作空间 —— 打开当前目录为 GitWorktree。
    ///
    /// 非 git 目录时静默跳过(用户可后续经命令打开其他目录)。
    fn init_default_workspace(&self) {
        if let Ok(cwd) = std::env::current_dir() {
            match GitWorktree::open(cwd) {
                Ok(wt) => {
                    self.manager
                        .add(Arc::new(wt) as Arc<dyn IWorkspace>);
                }
                Err(e) => {
                    log::info!("MainWindow: current dir is not a git worktree: {e}");
                }
            }
        }
    }

    /// 构建 ActivityBar + 激活首项 + observe active_id 同步 slot_left_size。
    ///
    /// 枚举已注册面板 → VisualActivityPanel 适配 → ActivityBar Entity。
    /// 面板经各扩展 crate `#[ctor::ctor]` + `register_activity_panel` 自注册,
    /// 此处经 `get_activity_panels()` 枚举并适配为 `IActivityPanel`。
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
}

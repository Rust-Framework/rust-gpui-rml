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
use rml_core::observable::ObservableVec;
use rml_core::value::IValue;
use rml_core::workbench::{IWorkbench, IWorkbenchManager, Uri};
use rust_dix::ServiceProvider;

use crate::di;
use crate::shell_manager::ArcShellManager;
use crate::welcome::register_welcome_abilities;

/// Arc Studio 主窗口 —— `#[window]` 声明式 GPUI 窗口。
///
/// - `manager` —— DI singleton,impl `IWorkbenchManager` + `IWorkspaceManager`
/// - `workbenches` —— 与 manager 共享的 `ObservableVec` 副本(版本号 + 数据共享)
/// - `activated` —— 与 manager 共享的 `Arc<RwLock<...>>`(同一 RwLock 实例)
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
            __rml_state: Default::default(),
        }
    }
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 1. 构建 DI 容器(注册 manager + WelcomeProvider,反向注入 provider 到 manager)
        let provider = match di::build_provider(self.manager.clone()) {
            Ok(p) => p,
            Err(e) => {
                log::error!("MainWindow: build DI provider failed: {e}");
                return;
            }
        };

        // 2. 注册 ServiceProvider 到 IAppContext(业务代码经 cx.get_service::<ServiceProvider>() 解析)
        cx.set_service::<ServiceProvider>(provider);

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

        // 4. 注册 welcome 能力(IContribution + IVisual + IWorkbench 能力 cast)
        register_welcome_abilities();

        // 5. 打开欢迎页(manager.open → push workbench → ObservableVec bump + flume send
        //    → 背景任务 cx.notify → #[computed] 重算 → TabBar 新增 Tab)
        let uri: Uri = "rml://welcome".parse().unwrap();
        if self.manager.open(&uri).is_some() {
            self.__rml_bump_version("activated");
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

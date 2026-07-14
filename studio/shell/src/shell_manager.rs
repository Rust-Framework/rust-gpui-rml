//! ArcShellManager —— 纯逻辑管理器(无 GPUI 依赖)。
//!
//! 同时 impl `IWorkbenchManager` + `IWorkspaceManager`,注册为 DI singleton。
//! `MainWindow`(GPUI Entity)持有 `Arc<ArcShellManager>` 用于 UI 渲染。
//!
//! # 二阶段注入
//!
//! `ArcShellManager::new()` 不带参数,创建后经 `set_provider()` 注入 ServiceProvider,
//! 解决"manager 需要 provider 解析子服务,provider 需要 manager 已注册"的循环依赖。

use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};

use rml_core::context::{IServiceProvider, ServiceProviderExt};
use rml_core::observable::ObservableVec;
use rml_core::workbench::{IWorkbench, IWorkbenchManager, IWorkbenchProvider, Uri};
use studio_core::workspace::{IWorkspace, IWorkspaceManager};

/// 纯逻辑管理器 —— 无 GPUI 依赖,可注册进 DI 容器。
///
/// 同时 impl IWorkbenchManager + IWorkspaceManager。
/// MainWindow(GPUI Entity)持有 Arc<ArcShellManager> 用于 UI 渲染。
pub struct ArcShellManager {
    /// DI 容器(二阶段注入) —— 用于解析 IWorkbenchProvider 等子服务。
    provider: OnceLock<Arc<dyn IServiceProvider + Send + Sync>>,
    /// 已打开的工作台会话(Tab)。带 flume 通知通道,push/remove 时 send。
    workbenches: ObservableVec<Arc<dyn IWorkbench>>,
    /// flume 接收端 —— MainWindow 在 on_loaded 中 clone 取走,驱动 cx.notify。
    notify_rx: flume::Receiver<()>,
    /// 当前激活的工作台。`Arc<RwLock<...>>` 共享给 MainWindow,避免镜像同步。
    activated: Arc<RwLock<Option<Arc<dyn IWorkbench>>>>,
    /// 已打开的工作空间(多根目录)。
    workspaces: RwLock<Vec<Arc<dyn IWorkspace>>>,
}

impl ArcShellManager {
    /// 创建管理器(不带 provider)。
    /// 构建后需调用 `set_provider()` 注入 DI 容器。
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();
        Self {
            provider: OnceLock::new(),
            workbenches: ObservableVec::with_notify(tx),
            notify_rx: rx,
            activated: Arc::new(RwLock::new(None)),
            workspaces: RwLock::new(Vec::new()),
        }
    }

    /// 二阶段注入 ServiceProvider(解决循环依赖)。
    pub fn set_provider(&self, provider: Arc<dyn IServiceProvider + Send + Sync>) {
        let _ = self.provider.set(provider);
    }

    /// 返回 workbenches 的克隆句柄(共享底层数据 + 版本号)。
    /// MainWindow 持有此克隆,#[computed] 经 version() 追踪变更。
    pub fn workbenches_handle(&self) -> ObservableVec<Arc<dyn IWorkbench>> {
        self.workbenches.clone()
    }

    /// 返回 flume 接收端克隆(MainWindow 背景任务 recv → cx.notify)。
    pub fn notify_receiver(&self) -> flume::Receiver<()> {
        self.notify_rx.clone()
    }

    /// 返回 activated 的共享句柄(同一 RwLock 实例)。
    /// MainWindow 持有此句柄,`#[computed] selected_tab` 直接读取,
    /// `on_tab_click` 直接写入 —— 无需镜像同步。
    pub fn activated_handle(&self) -> Arc<RwLock<Option<Arc<dyn IWorkbench>>>> {
        self.activated.clone()
    }

    fn provider(&self) -> &Arc<dyn IServiceProvider + Send + Sync> {
        self.provider
            .get()
            .expect("ServiceProvider not injected; call set_provider() after DI build")
    }
}

impl Default for ArcShellManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IWorkbenchManager for ArcShellManager {
    fn open(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();

        // 1. 去重:已打开则激活
        if let Some(wb) = self
            .workbenches
            .snapshot()
            .into_iter()
            .find(|w| w.uri() == uri_str)
        {
            *self.activated.write().unwrap() = Some(wb.clone());
            return Some(wb);
        }

        // 2. 路由:schema → DI keyed provider（经 ServiceProviderExt::get_keyed_trait）
        let schema = uri.scheme();
        let provider: Arc<dyn IWorkbenchProvider> =
            self.provider().get_keyed_trait::<dyn IWorkbenchProvider>(schema)?;
        let wb = provider.render(uri);

        // 3. 入栈 + 激活
        self.workbenches.push(wb.clone());
        *self.activated.write().unwrap() = Some(wb.clone());
        Some(wb)
    }

    fn close(&self, uri: &Uri) {
        let uri_str = uri.as_str();

        // 移除所有匹配 URI 的工作台(通常只有一个,while 兜底)
        while self.workbenches.remove_where(|w| w.uri() == uri_str) {}

        // 更新激活态:若关闭的是当前激活的,回退到第一个
        let mut activated = self.activated.write().unwrap();
        if activated
            .as_ref()
            .map(|w| w.uri() == uri_str)
            .unwrap_or(false)
        {
            *activated = self.workbenches.snapshot().into_iter().next();
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

impl IWorkspaceManager for ArcShellManager {
    fn add(&self, workspace: Arc<dyn IWorkspace>) {
        let root = workspace.root().to_path_buf();
        let mut ws = self.workspaces.write().unwrap();
        if !ws.iter().any(|w| w.root() == root) {
            ws.push(workspace);
        }
    }

    fn remove(&self, root: &Path) {
        self.workspaces
            .write()
            .unwrap()
            .retain(|w| w.root() != root);
    }

    fn list(&self) -> Vec<Arc<dyn IWorkspace>> {
        self.workspaces.read().unwrap().clone()
    }

    fn get(&self, root: &Path) -> Option<Arc<dyn IWorkspace>> {
        self.workspaces
            .read()
            .unwrap()
            .iter()
            .find(|w| w.root() == root)
            .cloned()
    }
}

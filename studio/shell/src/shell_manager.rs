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
        // Uri 三层识别:
        // 1. scheme → 选用 IWorkbenchProvider(下方 schema 路由)
        // 2. host + path → 资源唯一标识(去重依据,不含 query)
        // 3. query params → 传参参数(经 IWorkbench::set 传递,如 line=10 跳转)
        let target_id = resource_id(uri);

        // 1. 去重:已打开则激活 + 应用新 params(如新 line= 定位)
        if let Some(wb) = self
            .workbenches
            .snapshot()
            .into_iter()
            .find(|w| resource_id_of(w) == target_id)
        {
            *self.activated.write().unwrap() = Some(wb.clone());
            apply_query_params(&wb, uri);
            return Some(wb);
        }

        // 2. 路由:schema → DI keyed provider（经 ServiceProviderExt::get_keyed_trait）
        let schema = uri.scheme();
        let provider: Arc<dyn IWorkbenchProvider> =
            self.provider().get_keyed_trait::<dyn IWorkbenchProvider>(schema)?;
        let wb = provider.render(uri);

        // 3. 应用 url params(新打开的也应用,如 ?line=10&column=5)
        apply_query_params(&wb, uri);

        // 4. 入栈 + 激活
        self.workbenches.push(wb.clone());
        *self.activated.write().unwrap() = Some(wb.clone());
        Some(wb)
    }

    fn close(&self, uri: &Uri) {
        let target_id = resource_id(uri);

        // 移除所有匹配资源标识的工作台(通常只有一个,while 兜底)
        while self
            .workbenches
            .remove_where(|w| resource_id_of(w) == target_id)
        {}

        // 更新激活态:若关闭的是当前激活的,回退到第一个
        let mut activated = self.activated.write().unwrap();
        if activated
            .as_ref()
            .map(|w| resource_id_of(w) == target_id)
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
        let target_id = resource_id(uri);
        self.workbenches
            .snapshot()
            .into_iter()
            .find(|w| resource_id_of(w) == target_id)
    }

    fn open_preview(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let target_id = resource_id(uri);

        // 1. 已打开(不论预览/正式):激活 + 应用 params,不新建
        if let Some(wb) = self
            .workbenches
            .snapshot()
            .into_iter()
            .find(|w| resource_id_of(w) == target_id)
        {
            *self.activated.write().unwrap() = Some(wb.clone());
            apply_query_params(&wb, uri);
            return Some(wb);
        }

        // 2. 新打开:走 provider 路由 + 标记 preview
        let schema = uri.scheme();
        let provider: Arc<dyn IWorkbenchProvider> =
            self.provider().get_keyed_trait::<dyn IWorkbenchProvider>(schema)?;
        let wb = provider.render(uri);
        wb.set_preview(true);
        apply_query_params(&wb, uri);

        // 3. 单预览槽语义:若已有预览 Tab,替换之(保持原索引位置,VSCode 行为)
        //    replace_where 原子替换:一次 bump + 一次 flume send,无 UI 闪烁
        if self
            .workbenches
            .replace_where(|w| w.preview(), wb.clone())
            .is_none()
        {
            // 无已有预览 Tab,追加到末尾
            self.workbenches.push(wb.clone());
        }

        // 4. 激活
        *self.activated.write().unwrap() = Some(wb.clone());
        Some(wb)
    }

    fn promote(&self, uri: &Uri) {
        let target_id = resource_id(uri);
        if let Some(wb) = self
            .workbenches
            .snapshot()
            .into_iter()
            .find(|w| resource_id_of(w) == target_id)
        {
            if wb.preview() {
                wb.set_preview(false);
                // touch 触发 version bump + flume send → MainWindow computed 重算
                // (tab_items 读取 workbench.preview() 设置 TabItem.preview italic 视觉)
                self.workbenches.touch();
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Uri 三层识别辅助函数
// ──────────────────────────────────────────────────────────────────────────

/// 资源唯一标识:`scheme://host/path`(不含 query params)。
///
/// query params 是传参(如 `?line=10`),不参与资源身份判定。
/// 同一资源不同 params 视为同一 Tab(激活现有 + 应用新 params)。
///
/// 示例:
/// - `file:///e:/foo/bar.md` → `file:///e:/foo/bar.md`
/// - `file:///e:/foo/bar.md?line=10` → `file:///e:/foo/bar.md`(params 去除)
/// - `lsp://workspace/symbol/foo` → `lsp://workspace/symbol/foo`
fn resource_id(uri: &Uri) -> String {
    let scheme = uri.scheme();
    let host = uri.host_str().unwrap_or("");
    let path = uri.path();
    format!("{scheme}://{host}{path}")
}

/// 从 IWorkbench 解析资源标识。
///
/// IWorkbench::uri() 返回 &str,解析回 Url 提取 scheme+host+path。
/// 解析失败(业务返回非合法 Url)时回退到整串比对,保证兼容性。
fn resource_id_of(wb: &Arc<dyn IWorkbench>) -> String {
    let uri_str = wb.uri();
    match Uri::parse(uri_str) {
        Ok(u) => resource_id(&u),
        Err(_) => uri_str.to_string(),
    }
}

/// 解析 url query params 并经 `IWorkbench::set` 传递。
///
/// 如 `?line=10&column=5` → `set("line", "10")` + `set("column", "5")`。
/// IWorkbench 实现按需 downcast `String` 值处理业务语义(如跳转定位)。
///
/// # 何时调用
///
/// - 资源已打开:激活后应用新 params(如重新点击带 `?line=20` 的链接)
/// - 资源新打开:render 后立即应用 params
fn apply_query_params(wb: &Arc<dyn IWorkbench>, uri: &Uri) {
    for (key, value) in uri.query_pairs() {
        wb.set(key.into_owned().into(), Box::new(value.into_owned()));
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

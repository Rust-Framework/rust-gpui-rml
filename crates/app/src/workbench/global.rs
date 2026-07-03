//! 工作台管理器 App 扩展
//!
//! 框架内部：维护进程级 `Arc<dyn IWorkbenchManager>` 静态槽位；提供
//! `set_workbench_manager`/`get_workbench_manager` 扩展方法。
//!
//! 镜像 `ContributionRegistryExt`：OnceLock 进程级存储，`get_workbench_manager`
//! 返回 `Option<&'static dyn IWorkbenchManager>`，所有方法 `&self`，不借用 App。
//! manager 实现由业务提供，在启动时（如宿主 `on_loaded`）调用 `set_workbench_manager` 安装。

use std::sync::{Arc, OnceLock};

use gpui::App;
use rml_core::workbench::IWorkbenchManager;

static WORKBENCH_MANAGER: OnceLock<Arc<dyn IWorkbenchManager>> = OnceLock::new();

/// App 扩展：安装/获取 `IWorkbenchManager`。
pub trait WorkbenchManagerExt {
    /// 安装工作台管理器。仅首次调用生效；重复调用返回 `false`。
    fn set_workbench_manager(&self, manager: Arc<dyn IWorkbenchManager>) -> bool;

    /// 获取已安装的工作台管理器（`&'static`，不借用 App）。
    fn get_workbench_manager(&self) -> Option<&'static dyn IWorkbenchManager>;
}

impl WorkbenchManagerExt for App {
    fn set_workbench_manager(&self, manager: Arc<dyn IWorkbenchManager>) -> bool {
        WORKBENCH_MANAGER.set(manager).is_ok()
    }

    fn get_workbench_manager(&self) -> Option<&'static dyn IWorkbenchManager> {
        WORKBENCH_MANAGER.get().map(|a| a.as_ref())
    }
}

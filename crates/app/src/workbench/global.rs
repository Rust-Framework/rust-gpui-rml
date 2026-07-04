//! 工作台管理器注册槽位
//!
//! `WorkbenchManagerSlot` 是 newtype 包装 `Arc<dyn IWorkbenchManager>` 以便存入
//! `ServiceCollection`（因为 `Arc<dyn Trait>` 不能直接作为 `T: 'static + Send + Sync`
//! 泛型参数被 downcast）。业务通过 `IAppContextExt::set_workbench_manager` /
//! `workbench_manager()` 操作。

use std::sync::Arc;

use rml_core::workbench::IWorkbenchManager;

/// 工作台管理器注册槽位（newtype 包装 `Arc<dyn IWorkbenchManager>` 以便存入 `ServiceCollection`）。
pub struct WorkbenchManagerSlot(pub Arc<dyn IWorkbenchManager + Send + Sync>);

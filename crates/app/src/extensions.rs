//! RML App/Context 扩展中央聚合点
//!
//! 统一 re-export 所有 App/Context 扩展 trait，并提供 `IAppContextExt` 便利方法（语法糖）。
//! 业务代码只需 `use rml_app::prelude::*` 即可获得全部扩展方法。
//!
//! 设计层次：
//! - `IAppContext`（rml_core）：核心 IServiceProvider 接口，3 方法
//! - `IAppContextExt`（本模块）：常用服务的语义化便利方法，转发到 `IAppContext::get_service`
//! - `I18nExt` / `ThemeExt`（rml_core）：领域特定状态操作（带副作用，不通过 IServiceProvider）

use std::sync::Arc;

use gpui::App;
use rml_core::contribution::IContributionRegistry;
use rml_core::context::IAppContext;
use rml_core::workbench::IWorkbenchManager;

use crate::contribution::ContributionRegistry;
use crate::workbench::WorkbenchManagerSlot;

/// IAppContext 便利方法——为常用服务提供语义化访问。
///
/// 这些方法是 `IAppContext::get_service::<T>()` 的语法糖，
/// 不引入新的存储机制，仅转发到 `ServiceCollection`。
pub trait IAppContextExt: IAppContext {
    /// 获取贡献注册表（必需服务）。
    fn contribution_registry(&self) -> Arc<ContributionRegistry> {
        self.get_required_service::<ContributionRegistry>()
    }

    /// 获取贡献注册表的 trait object 视图。
    fn contribution_registry_dyn(&self) -> Arc<dyn IContributionRegistry> {
        self.contribution_registry() as Arc<dyn IContributionRegistry>
    }

    /// 安装工作台管理器。
    fn set_workbench_manager(&mut self, manager: Arc<dyn IWorkbenchManager + Send + Sync>) {
        self.set_service(Arc::new(WorkbenchManagerSlot(manager)));
    }

    /// 获取已安装的工作台管理器。
    fn workbench_manager(&self) -> Option<Arc<dyn IWorkbenchManager + Send + Sync>> {
        self.get_service::<WorkbenchManagerSlot>().map(|s| s.0.clone())
    }
}

impl IAppContextExt for App {}

// 重新导出所有 App/Context 扩展 trait，构成中央聚合点
// IAppContext 由 lib.rs 直接从 rml_core::context 导入（避免与本模块 use 冲突）
pub use rml_core::command::CommandAbilityExt;
pub use rml_core::context::{ensure_service_collection, ServiceCollection};
pub use rml_core::contribution::{ContributionAbilityExt, VisualAbilityExt};
pub use rml_core::i18n::I18nExt;
pub use rml_core::theme::ThemeExt;

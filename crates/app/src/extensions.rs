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
use rml_core::contribution::{IContributionHost, IContributionRegistry};
use rml_core::context::IAppContext;

use crate::contribution::ContributionRegistry;

/// IAppContext 便利方法——为常用服务提供语义化访问。
///
/// 这些方法是 `IAppContext::get_service::<T>()` 的语法糖，
/// 不引入新的存储机制，仅转发到 `ServiceCollection`。
pub trait IAppContextExt: IAppContext {
    /// 获取贡献注册表（trait object 视图，隐藏具体类型）。
    fn get_contribution_registry(&self) -> Arc<dyn IContributionRegistry> {
        self.get_required_service::<ContributionRegistry>() as Arc<dyn IContributionRegistry>
    }

    /// 注册 host。`host` 经 `Arc<dyn IContributionHost>` 提供 `add`/`remove` 能力，
    /// registry 调用 trait 方法路由贡献，不依赖具体存储类型（依赖倒置）。
    ///
    /// 默认用法：host 持有 `entries: Arc<RwLock<Vec<...>>>` 共享存储（即 `ContributionStorage`），
    /// `entries.clone()` 经 unsized coercion 转为 `Arc<dyn IContributionHost>`——
    /// 业务代码无需自定义 host 类型。需要自定义受理逻辑时，为自身类型 impl `IContributionHost`。
    fn register_host(&self, host_id: &str, host: Arc<dyn IContributionHost>) {
        self.get_contribution_registry().add(host_id, host);
    }
}

impl IAppContextExt for App {}

// 重新导出所有 App/Context 扩展 trait，构成中央聚合点
// IAppContext 由 lib.rs 直接从 rml_core::context 导入（避免与本模块 use 冲突）
pub use rml_core::command::CommandAbilityExt;
pub use rml_core::context::{ensure_service_collection, ServiceCollection};
pub use rml_core::contribution::{ContributionAbilityExt, StatusBarItemAbilityExt, VisualAbilityExt};
pub use rml_core::i18n::I18nExt;
pub use rml_core::theme::ThemeExt;

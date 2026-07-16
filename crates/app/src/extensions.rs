//! RML App/Context 扩展中央聚合点
//!
//! 统一 re-export 所有 App/Context 扩展 trait，并提供 `IAppContextExt` 便利方法（语法糖）。
//! 业务代码只需 `use rml_app::prelude::*` 即可获得全部扩展方法。
//!
//! 设计层次：
//! - `IAppContext`（crate::context）：核心 IServiceProvider 接口（产品层 DI 集成）
//! - `IAppContextExt`（本模块）：框架内部服务的语义化便利方法，经 GPUI Global 存取
//! - `I18nExt` / `ThemeExt`（rml_core）：领域特定状态操作（带副作用，GPUI Global）

use std::sync::Arc;

use gpui::{App, AppContext};
use rml_core::contribution::{IContributionHost, IContributionRegistry};

use crate::context::IAppContext;
use crate::contribution::ContributionRegistry;

/// IAppContext 便利方法——为框架内部服务提供语义化访问。
///
/// 这些方法经 GPUI Global 存取（不经过 IServiceProvider），适用于框架自身服务。
/// 产品层 DI 服务（IWorkbenchManager 等）仍经 `IAppContext::get_service` 查询。
pub trait IAppContextExt: IAppContext {
    /// 获取贡献注册表（trait object 视图，隐藏具体类型）。
    ///
    /// ContributionRegistry 经 GPUI Global 存储（`set_global`），不经过 IServiceProvider。
    /// 返回 `Arc<dyn IContributionRegistry>` 以隐藏具体类型；
    /// 内部 `ContributionRegistry` 为 newtype(`Arc<Inner>`)，`clone` 为浅拷贝。
    ///
    /// 使用 `AppContext::read_global`（trait 方法）而非 `App::global`（inherent 方法），
    /// 以便在 trait 默认方法中通过 `where Self: AppContext` 约束调用。
    fn get_contribution_registry(&self) -> Arc<dyn IContributionRegistry>
    where
        Self: Sized + AppContext,
    {
        self.read_global::<ContributionRegistry, _>(|registry, _app| {
            Arc::new(registry.clone()) as Arc<dyn IContributionRegistry>
        })
    }

    /// 注册 host。`host` 经 `Arc<dyn IContributionHost>` 提供 `add`/`remove` 能力，
    /// registry 调用 trait 方法路由贡献，不依赖具体存储类型（依赖倒置）。
    ///
    /// 默认用法：host 持有 `entries: Arc<RwLock<Vec<...>>>` 共享存储（即 `ContributionStorage`），
    /// `entries.clone()` 经 unsized coercion 转为 `Arc<dyn IContributionHost>`——
    /// 业务代码无需自定义 host 类型。需要自定义受理逻辑时，为自身类型 impl `IContributionHost`。
    fn register_host(&self, host_id: &str, host: Arc<dyn IContributionHost>)
    where
        Self: Sized + AppContext,
    {
        self.get_contribution_registry().add(host_id, host);
    }
}

impl IAppContextExt for App {}

// 重新导出所有 App/Context 扩展 trait，构成中央聚合点
pub use crate::context::{ensure_service_provider, IServiceProvider, RuntimeServiceRegistry, resolve_service, resolve_keyed_service, resolve_required_service, resolve_required_keyed_service};
pub use rml_core::command::CommandAbilityExt;
pub use rml_core::contribution::{ContributionAbilityExt, StatusBarItemAbilityExt, VisualAbilityExt};
pub use rml_core::i18n::I18nExt;
pub use rml_core::theme::ThemeExt;

//! RML 应用层 prelude——业务代码 `use rml_app::prelude::*` 获得全部扩展
//!
//! 包含：
//! - `IAppContext` / `IServiceProvider` 核心接口（统一服务访问）
//! - `IAppContextExt` 便利方法（contribution_registry 语法糖）
//! - `I18nExt` / `ThemeExt` 领域特定状态操作
//! - `IAppLifecycle` 应用生命周期接口
//! - 能力查询扩展（VisualAbilityExt / ContributionAbilityExt / CommandAbilityExt）

pub use crate::context::{
    ensure_service_provider, IAppContext, IServiceProvider, RuntimeServiceRegistry,
    resolve_service, resolve_keyed_service, resolve_required_service, resolve_required_keyed_service,
};
pub use crate::extensions::*;
pub use crate::lifecycle::IAppLifecycle;

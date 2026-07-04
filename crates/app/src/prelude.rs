//! RML 应用层 prelude——业务代码 `use rml_app::prelude::*` 获得全部扩展
//!
//! 包含：
//! - `IAppContext` 核心接口（IServiceProvider 风格统一服务访问）
//! - `IAppContextExt` 便利方法（contribution_registry / workbench_manager 语法糖）
//! - `I18nExt` / `ThemeExt` 领域特定状态操作
//! - `IAppLifecycle` 应用生命周期接口
//! - 能力查询扩展（VisualAbilityExt / ContributionAbilityExt / CommandAbilityExt）

pub use crate::extensions::*;
pub use crate::lifecycle::IAppLifecycle;

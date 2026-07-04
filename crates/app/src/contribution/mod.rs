//! 贡献点运行时：Registry + 视觉 Entity 缓存
//!
//! 框架内部模块，业务代码通过 `rml_app::prelude::*` 或具体导入使用。
//! 注册表实例通过 `IAppContext::get_service::<ContributionRegistry>()` 查询。
//! Host 直接实现 `IContributionHost`，在 `on_loaded` 中调
//! `cx.get_contribution_registry().add(...)` + `bootstrap_host_contributions(cx, id)` 注册自身。

mod entity_cache;
mod global;
mod registry;

pub use entity_cache::{get_or_create_entity, visual_entity, VisualEntityCache};
pub use global::{bootstrap_host_contributions, install_contribution_bootstrap};
pub use registry::ContributionRegistry;

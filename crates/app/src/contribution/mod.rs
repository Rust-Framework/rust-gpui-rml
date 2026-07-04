//! 贡献点运行时：Registry + Entity host 桥接
//!
//! 框架内部模块，业务代码通过 `rml_app::prelude::*` 或具体导入使用。
//! 注册表实例通过 `IAppContext::get_service::<ContributionRegistry>()` 查询。

mod entity_cache;
mod global;
mod host_handle;
mod registry;

pub use entity_cache::{get_or_create_entity, visual_entity, VisualEntityCache};
pub use global::{bootstrap_host_contributions, install_contribution_bootstrap};
pub use host_handle::{drain_host_ops, install_entity_host, EntityHostHandle, HostOp};
pub use registry::ContributionRegistry;

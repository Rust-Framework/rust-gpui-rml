//! 贡献点运行时：Registry + Entity host 桥接
//!
//! 框架内部模块，业务代码通过 `rml_app::prelude::*` 或具体导入使用。

mod entity_cache;
mod global;
mod host_handle;
mod registry;

pub use entity_cache::{get_or_create_entity, visual_entity};
pub use global::{
    bootstrap_host_contributions, ensure_contribution_registry, install_contribution_bootstrap,
    ContributionRegistryExt,
};
pub use host_handle::{drain_host_ops, install_entity_host, EntityHostHandle, HostOp};
pub use registry::ContributionRegistry;

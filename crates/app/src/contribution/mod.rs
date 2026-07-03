//! 贡献点运行时：Registry + 视觉提取器
//!
//! 框架内部模块，业务代码通过 `rml_app::prelude::*` 或具体导入使用。

mod entity_cache;
mod global;
mod registry;

pub use entity_cache::{build_activity_panels, get_or_create_entity, visual_entity};
pub use global::{
    bootstrap_contributions, ensure_contribution_registry, install_contribution_bootstrap,
    ContributionRegistryExt,
};
pub use registry::extract_visual;

#[doc(hidden)]
pub use registry::{register_visual_extractor, ContributionRegistry};

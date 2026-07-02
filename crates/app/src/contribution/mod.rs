//! 贡献点运行时：Registry + host 注册 + 视觉提取器
//!
//! 框架内部模块，业务代码通过 `rml_app::prelude::*` 或具体导入使用。

mod global;
mod registry;

pub use global::{
    bootstrap_contributions, ensure_contribution_registry, install_contribution_bootstrap,
    register_host, ContributionRegistryExt,
};
pub use registry::extract_visual;

#[doc(hidden)]
pub use global::EntityHostHandleBox;
#[doc(hidden)]
pub use registry::{register_visual_extractor, ContributionRegistry};

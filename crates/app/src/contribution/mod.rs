//! 贡献点运行时：Registry / Host / Entity 缓存

mod entry;
mod global;
mod host;
mod registerable;
mod registry;
mod tree;

pub use entry::{data_entry, data_entry_dyn, visual_entry};
pub use global::{ensure_contribution_registry, ContributionExt, ContributionRegistryGlobal};
pub use host::ContributionHost;
pub use registerable::{data_registerable, Registerable};
pub use registry::ContributionRegistry;
pub use tree::{build_contribution_tree, ContributionTreeNode};

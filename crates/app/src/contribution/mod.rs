//! 贡献点运行时：Registry / Host / Entity 缓存

mod entry;
mod global;
mod host;
mod host_view;
mod registerable;
mod registry;
mod tree;

pub use entry::{data_entry, data_entry_dyn, visual_entry};
pub use global::{
    bootstrap_contributions, contribution_entries, ensure_contribution_registry,
    install_contribution_bootstrap, register_contribution, ContributionExt,
    ContributionRegistryGlobal,
};
pub use host::{box_host, ContributionHost};
pub use host_view::{attach_host_view, ContributionHostView};
pub use registerable::{data_registerable, visual_registerable, Registerable};
pub use registry::ContributionRegistry;
pub use tree::{build_contribution_tree, ContributionTreeNode};

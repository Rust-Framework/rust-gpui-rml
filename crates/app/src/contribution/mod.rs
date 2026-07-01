//! 贡献点运行时：Registry + 注册/bootstrap

mod activity_panel;
mod entry;
mod global;
mod host;
mod registerable;
mod registry;
mod render;

pub use activity_panel::{map_activity_panels, resolve_active_panel_body};
pub use entry::{component_entry, data_entry, data_entry_dyn};
pub use global::{
    bootstrap_contributions, contribution_entries, contribution_revision,
    ensure_contribution_registry, install_contribution_bootstrap, register_contribution,
    subscribe_host_changes, ContributionExt,
};
#[doc(hidden)]
pub use global::ContributionRegistryGlobal;
pub use host::ContributionHost;
pub use registerable::{component_registerable, data_registerable, Registerable};
#[doc(hidden)]
pub use registry::ContributionRegistry;
#[doc(hidden)]
pub use render::render_contribution_visual;

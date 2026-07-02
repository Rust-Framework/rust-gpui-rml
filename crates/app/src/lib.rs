//! RML 应用启动器
//!
//! 提供 `RmlApplication` 作为应用入口，封装 GPUI 的窗口创建与生命周期管理。

#![forbid(unsafe_code)]

extern crate rust_rml_core as rml_core;

pub mod application;
pub mod contribution;
pub mod lifecycle;
pub mod resources;

pub use application::{NoWindow, RmlApplication};
pub use contribution::{
    bootstrap_contributions, ensure_contribution_registry, extract_visual, register_host,
    ContributionRegistryExt,
};
pub use lifecycle::IAppLifecycle;
pub use resources::{
    load_i18n_catalog, load_i18n_from_json, load_theme_colors, load_theme_css,
    DEFAULT_I18N_DIR, DEFAULT_THEMES_DIR,
};

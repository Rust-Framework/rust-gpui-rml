//! RML 应用启动器
//!
//! 提供 `RmlApplication` 作为应用入口，封装 GPUI 的窗口创建与生命周期管理。
//! 通过 `IAppContext`（IServiceProvider 风格）统一全局服务访问。

#![forbid(unsafe_code)]

extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

pub mod application;
pub mod assets;
pub mod context;
pub mod contribution;
pub mod extensions;
pub mod lifecycle;
pub mod resources;

pub mod prelude;

pub use application::{NoWindow, RmlApplication};
pub use context::{
    ensure_service_provider, IAppContext, IServiceProvider, RuntimeServiceRegistry,
    resolve_service, resolve_keyed_service, resolve_required_service, resolve_required_keyed_service,
};
pub use lifecycle::IAppLifecycle;
pub use resources::{
    load_i18n_catalog, load_i18n_from_json, load_theme_colors, load_theme_css,
    DEFAULT_I18N_DIR, DEFAULT_THEMES_DIR,
};

// IAppContext 核心 + 便利方法
pub use extensions::IAppContextExt;

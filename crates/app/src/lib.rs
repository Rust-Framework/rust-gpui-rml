//! RML 应用启动器
//!
//! 提供 `RmlApplication` 作为应用入口，封装 GPUI 的窗口创建与生命周期管理。
//!
//! ## 双入口使用模式
//!
//! - **声明式**：`RmlApplication::new().main_window::<W>().run()`（WPF StartupUri 风格）
//! - **命令式**：`RmlApplication::new().run::<A>()`（WPF OnStartup 重写风格）
//!
//! `app` crate **不依赖** `ui` crate。`IWindow` trait 定义在 `core` crate，
//! 窗口打开逻辑由 `W` 的 `IWindow::open()` 实现负责（在 `ui` crate 或用户代码中）。

#![forbid(unsafe_code)]

// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_core as rml_core;

pub mod application;
pub mod lifecycle;
pub mod resources;

pub use application::{NoWindow, RmlApplication};
pub use lifecycle::{IAppLifecycle, NoLifecycle};
pub use resources::{
    load_i18n_catalog, load_i18n_from_json, load_theme_colors, load_theme_css,
    DEFAULT_I18N_DIR, DEFAULT_THEMES_DIR,
};

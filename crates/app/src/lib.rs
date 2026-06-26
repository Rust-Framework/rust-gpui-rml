//! RML 应用启动器与窗口管理
//!
//! 提供 `RmlApplication` 作为应用入口，封装 GPUI 的窗口创建与生命周期管理。
//!
//! ## Feature: `ui-components`（默认开启）
//!
//! 启用后：
//! - 在 `Application::run` 启动时调用 `rml_ui::init(cx)` 初始化 gpui-component 全局状态
//! - 窗口顶层使用 `rml_ui::Root` 包裹业务 view，从而支持 Dialog/Sheet/Notification 等浮层
//!
//! 关闭后退化为「裸 GPUI 窗口」，业务 view 直接作为窗口根 view。

#![forbid(unsafe_code)]

// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_core as rml_core;

#[cfg(feature = "ui-components")]
extern crate rust_rml_ui as rml_ui;

pub mod application;
pub mod resources;
pub mod window;

pub use application::RmlApplication;

//! RML 应用启动器与窗口管理
//!
//! 提供 `RmlApplication` 作为应用入口，封装 GPUI 的窗口创建与生命周期管理。

#![forbid(unsafe_code)]

pub mod application;
pub mod resources;
pub mod window;

pub use application::RmlApplication;

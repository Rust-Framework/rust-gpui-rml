//! 内置窗口类型 —— 开箱即用的 `IWindow` 实现
//!
//! 提供 `Window`（基础窗口）和 `ModernWindow`（带 chrome 的现代窗口）。
//! 类比 WPF `Window` 类：可直接使用，也可作为更复杂窗口的基础。
//!
//! 用户创建带 RML 模板的窗口应使用 `#[window]` 宏。
//! 内置窗口适用于简单场景（占位窗口、启动画面、关于对话框等）。
//!
//! # 示例
//!
//! ```rust,ignore
//! use rml_ui::prelude::*;
//! use rml_app::RmlApplication;
//!
//! fn main() {
//!     RmlApplication::new()
//!         .main_window::<rml_ui::ModernWindow>()
//!         .run();
//! }
//! ```

mod basic;
mod modern;

pub use basic::Window;
pub use modern::ModernWindow;

//! 窗口组件模块
//!
//! 提供 ModernWindowShell / TabWindowShell 内置封装组件 + IWindowActions 助手 trait。
//! 提供内置 Window/ModernWindow IWindow 实现用于开箱即用的窗口创建。
//!
//! ModernWindowShell / TabWindowShell 是易用性封装，内置组合 TitleBar + 插槽 + StatusBar。
//! 菜单 / 状态栏通过**Vue 风格插槽扩展**（`<template slot="menu">` / `<template slot="footer">`）传入，
//! 不再提供框架级 `MenuItem` / `StatusBarItem` 数据结构。
//! 用户在 RML 中用 `<MenuItem label="..." command={field} />` 声明菜单结构，
//! ViewModel 仅持有 `Arc<dyn ICommand>` 字段，绑定到控件 click。

pub mod actions;
pub mod builtin_window;
pub mod ext;
pub mod modern_window;
pub mod tab_window;

pub use actions::{IWindowActions, NotificationKind};
pub use builtin_window::{ModernWindow, Window};
pub use ext::IWindowExt;
pub use modern_window::ModernWindowShell;
pub use tab_window::TabWindowShell;

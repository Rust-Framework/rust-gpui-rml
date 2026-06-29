//! 窗口组件模块
//!
//! 提供 ModernWindowShell 内置封装组件 + MenuItem/StatusBarItem 数据类型 + IWindowActions 助手 trait。
//! 提供内置 Window/ModernWindow IWindow 实现用于开箱即用的窗口创建。
//!
//! ModernWindowShell 是易用性封装，内置组合 TitleBar + Menu + StatusBar。
//! 用户也可选择手动组装原子组件：`<TitleBar>` / `<StatusBar>` / `<Kbd>`。

pub mod actions;
pub mod builtin_window;
pub mod ext;
pub mod menu_bar;
pub mod modern_window;
pub mod types;

pub use actions::{IWindowActions, NotificationKind};
pub use builtin_window::{ModernWindow, Window};
pub use ext::IWindowExt;
pub use modern_window::ModernWindowShell;
pub use types::{MenuItem, StatusBarItem};

//! 窗口组件模块
//!
//! 提供 ModernWindow 内置封装组件 + MenuItem/StatusBarItem 数据类型 + IWindowActions 助手 trait。
//!
//! ModernWindow 是易用性封装，内置组合 TitleBar + Menu + StatusBar。
//! 用户也可选择手动组装原子组件：`<TitleBar>` / `<StatusBar>` / `<Kbd>`。

pub mod actions;
pub mod menu_bar;
pub mod modern_window;
pub mod types;

pub use actions::{IWindowActions, NotificationKind};
pub use modern_window::ModernWindow;
pub use types::{MenuItem, StatusBarItem};

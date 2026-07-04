//! `StatusBar` —— 状态栏对齐枚举 + gpui-component `NativeStatusBar` re-export。
//!
//! 框架不定义 `IStatusBarItem` 数据结构（WPF 风格——业务定义自己的 ViewModel）。
//! 业务侧 `MainWindow::render_status_bar()` 经 `NativeStatusBar::new()` + `.left()` / `.right()`
//! / `.child()` 组装，对齐信息由 `StatusBarAlign` 表达。

pub use gpui_component::status_bar::StatusBar as NativeStatusBar;

/// 状态栏项对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarAlign {
    Left,
    Right,
    Center,
}

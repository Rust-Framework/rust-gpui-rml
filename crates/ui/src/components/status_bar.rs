//! `StatusBar` —— 状态栏对齐枚举 + gpui-component `NativeStatusBar` re-export。
//!
//! `StatusBarAlign` 已移至 `rml_core::contribution`(因 `IStatusBarItem::align()` 返回类型
//! 需要在 core 定义)。本模块经 `pub use` re-export 保持 `rml_ui::StatusBarAlign` 兼容。
//!
//! 框架提供 `IStatusBarItem` trait(仅 `align()`)作为状态栏容器的数据契约。
//! 业务侧 `MainWindow::render_status_bar()` 经 `NativeStatusBar::new()` + `.left()` / `.right()`
//! / `.child()` 组装,对齐信息由 `IStatusBarItem::align()` 提取。

pub use gpui_component::status_bar::StatusBar as NativeStatusBar;

pub use rml_core::contribution::StatusBarAlign;

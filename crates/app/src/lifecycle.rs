//! `IAppLifecycle` trait —— 应用级生命周期契约
//!
//! 类比 WPF 的 `Application` 类。配合 `RmlApplication` 声明式入口使用：
//!
//! ```rust,ignore
//! fn main() {
//!     rml_app::RmlApplication::new()
//!         .main_window::<MainWindow>()
//!         .run::<Startup>();
//! }
//!
//! impl IAppLifecycle for Startup {
//!     fn on_launch(&mut self, cx: &mut App) {
//!         cx.set_style("styles.css");
//!         cx.set_i18n("zh-CN");
//!     }
//!
//!     fn on_main_window_ready(&mut self, window: &mut Window, cx: &mut App) {
//!         LoginDialog::default().open(window, cx);
//!     }
//! }
//! ```

use gpui::{App, Window};

/// 应用级生命周期 trait
///
/// 执行顺序（声明式入口）：
/// 1. `on_launch` —— 全局初始化（style / i18n / theme），主窗口尚未创建
/// 2. 框架自动打开主窗口 `W::default().open(cx)`
/// 3. `on_main_window_ready` —— 主窗口就绪，可在其上弹出 Dialog / Sheet
pub trait IAppLifecycle: Sized + Send + Default + 'static {
    /// 应用启动时调用（仅一次），在主窗口 `open()` **之前**执行。
    ///
    /// 典型用途：`cx.set_style` / `cx.set_i18n` / `cx.set_theme` / 注册全局 Action。
    fn on_launch(&mut self, cx: &mut App);

    /// 主窗口就绪后调用（声明式入口），拥有 `&mut Window` 可打开 Dialog / Sheet。
    fn on_main_window_ready(&mut self, _window: &mut Window, _cx: &mut App) {}

    fn on_exit(&mut self, _cx: &mut App) {}
    fn on_activate(&mut self, _cx: &mut App) {}
    fn on_deactivate(&mut self, _cx: &mut App) {}
}

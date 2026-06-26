//! `IAppLifecycle` trait —— 应用级生命周期契约
//!
//! 类比 WPF 的 `Application` 类：由 App 负责打开主窗口，而非直接绑定视图类型。
//! `RmlApplication::run::<A>()` 中 `A` 必须实现此 trait。
//!
//! ## 典型用法
//!
//! ```rust,ignore
//! use rml_app::{IAppLifecycle, RmlApplication, ModernWindow};
//! use gpui::App;
//!
//! struct MyApp;
//!
//! impl IAppLifecycle for MyApp {
//!     fn on_launch(&mut self, cx: &mut App) {
//!         ModernWindow::new("My App", gpui::px(800.), gpui::px(600.))
//!             .open::<MyView>(cx);
//!     }
//! }
//!
//! fn main() {
//!     RmlApplication::new().run::<MyApp>();
//! }
//! ```

use gpui::App;

/// 应用级生命周期 trait
///
/// `RmlApplication::run::<A>()` 中 `A` 必须实现此 trait。
/// 类比 WPF 的 `Application` 类，由 App 负责打开主窗口，而非直接绑定视图类型。
///
/// 用户只需实现 `on_launch`，其它回调可选。
pub trait IAppLifecycle: Sized + Send + 'static {
    /// 应用启动时调用（仅一次）
    ///
    /// 典型用途：打开主窗口、初始化全局状态、注册 app 级 Action。
    /// 在此处调用 `rml_app::Window::new(...).open::<MyView>(cx)` 或
    /// `rml_app::ModernWindow::new(...).open::<MyView>(cx)`。
    fn on_launch(&mut self, cx: &mut App);

    /// 应用退出前调用（仅一次）
    ///
    /// 典型用途：保存状态、释放资源。
    fn on_exit(&mut self, _cx: &mut App) {}

    /// 应用被激活（前台）时调用
    fn on_activate(&mut self, _cx: &mut App) {}

    /// 应用被停用（后台）时调用
    fn on_deactivate(&mut self, _cx: &mut App) {}
}

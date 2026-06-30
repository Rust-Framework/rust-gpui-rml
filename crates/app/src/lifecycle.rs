//! `IAppLifecycle` trait —— 应用级生命周期契约
//!
//! 类比 WPF 的 `Application` 类。
//!
//! ## 双入口
//!
//! **命令式** `RmlApplication::new().run::<MyApp>()`：
//! - `on_launch` 拥有**完全控制权**——初始化 i18n、打开登录/欢迎窗、注册全局状态
//! - **不会**自动打开主窗口；主窗口在登录成功等回调里手动 `open()`
//!
//! **声明式** `RmlApplication::new().lifecycle::<Hooks>().main_window::<W>().run()`：
//! - 先执行 `Hooks::on_launch`（如 `cx.use_i18n`），再自动打开 `W`
//! - 适合「启动配置 + 单主窗口」的简单应用
//!
//! ## 典型：先登录/欢迎，再主窗口（命令式）
//!
//! ```rust,ignore
//! impl IAppLifecycle for MyApp {
//!     fn on_launch(&mut self, cx: &mut App) {
//!         cx.use_i18n("zh-CN");
//!         LoginWindow::default().open(cx);
//!         // 登录成功后：MainWindow::default().open(cx);
//!     }
//! }
//! ```

use gpui::App;

/// 应用级生命周期 trait
pub trait IAppLifecycle: Sized + Send + Default + 'static {
    /// 应用启动时调用（仅一次）
    ///
    /// 典型用途：
    /// - `cx.use_i18n("zh-CN")` / `cx.use_i18n_with_dir(...)`
    /// - 打开登录窗、欢迎窗（命令式入口）
    /// - 注册全局状态、app 级 Action
    ///
    /// 声明式入口下，此方法在主窗口 `open()` **之前**执行。
    fn on_launch(&mut self, cx: &mut App);

    fn on_exit(&mut self, _cx: &mut App) {}
    fn on_activate(&mut self, _cx: &mut App) {}
    fn on_deactivate(&mut self, _cx: &mut App) {}
}

/// 标记：声明式入口未设置 lifecycle 钩子
#[derive(Default)]
pub struct NoLifecycle;

impl IAppLifecycle for NoLifecycle {
    fn on_launch(&mut self, _cx: &mut App) {}
}

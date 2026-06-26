//! `RmlApplication` —— 应用启动器
//!
//! 封装 GPUI 的 `Application`，提供 `RmlApplication::new().run::<A>()` API，
//! 其中 `A: IAppLifecycle`。窗口创建权交给 App（类比 WPF `Application`）。
//!
//! ## Feature `ui-components`（默认开启）
//!
//! 启用时：在 `Application::run` 启动回调中调用 `rml_ui::init(cx)` 初始化
//! gpui-component 全局状态（theme / global_state / root / dialog / ...）。
//!
//! 关闭时：不引入 gpui-component 依赖，App 需自行管理窗口。
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

use crate::lifecycle::IAppLifecycle;

/// RML 应用启动器
///
/// 不再直接绑定视图类型，而是由 `A: IAppLifecycle` 控制窗口创建。
/// 这类比 WPF 的 `Application` 类：App 负责打开主窗口，而非框架自动创建。
pub struct RmlApplication;

impl RmlApplication {
    pub fn new() -> Self {
        Self
    }

    /// 启动应用，由 `A: IAppLifecycle` 控制窗口创建与生命周期。
    ///
    /// `A` 必须实现 `IAppLifecycle`（含 `on_launch`）和 `Default`。
    /// 在 `on_launch` 中调用 `rml_app::Window::new(...).open::<V>(cx)` 或
    /// `rml_app::ModernWindow::new(...).open::<V>(cx)` 打开主窗口。
    pub fn run<A>(self)
    where
        A: IAppLifecycle + Default + 'static,
    {
        gpui_platform::application().run(move |cx: &mut App| {
            // 初始化 gpui-component 全局状态：theme / global_state / root / dialog / ...
            // 必须在打开窗口前完成
            #[cfg(feature = "ui-components")]
            rml_ui::init(cx);

            // 创建 App 实例并触发 on_launch —— 由 App 负责打开主窗口
            let mut app = A::default();
            app.on_launch(cx);

            // Phase 4：注册 on_exit / on_activate / on_deactivate 回调
            // GPUI 当前没有显式的 on_exit 钩子，可在 on_release 中近似处理
        });
    }
}

impl Default for RmlApplication {
    fn default() -> Self {
        Self::new()
    }
}

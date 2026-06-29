//! `RmlApplication` —— 应用启动器
//!
//! 参考 WPF `Application` 类设计：
//! - `main_window::<W: IWindow>()` 设置主窗口类型（内置功能，非扩展）
//! - `run()` 启动应用并打开主窗口（声明式）或由 `IAppLifecycle` 控制（命令式）
//!
//! ## 双入口使用模式
//!
//! **模式 A：声明式（WPF StartupUri 风格，推荐）**
//!
//! ```rust,ignore
//! use rml_app::RmlApplication;
//! use rml_ui::prelude::*;  // 启用 #[window] 宏 + 内置组件
//!
//! #[window(title = "My App", width = 800, height = 600)]
//! #[derive(Default)]
//! pub struct MainWindow {
//!     pub count: i32,
//! }
//!
//! fn main() {
//!     RmlApplication::new()
//!         .main_window::<MainWindow>()  // 内置方法，无需 Ext trait
//!         .run();
//! }
//! ```
//!
//! **模式 B：命令式（WPF OnStartup 重写风格）**
//!
//! ```rust,ignore
//! use rml_app::{IAppLifecycle, RmlApplication};
//! use gpui::App;
//!
//! struct MyApp;
//!
//! impl IAppLifecycle for MyApp {
//!     fn on_launch(&mut self, cx: &mut App) {
//!         // 手动创建并打开窗口
//!     }
//! }
//!
//! fn main() {
//!     RmlApplication::new().run::<MyApp>();
//! }
//! ```

use std::marker::PhantomData;

use gpui::App;
use rml_core::window::IWindow;

use crate::lifecycle::IAppLifecycle;

/// 标记类型：未设置主窗口（不可实现 `IWindow`，避免 impl 冲突）
pub struct NoWindow;

/// RML 应用启动器
///
/// 内置主窗口设置，无需扩展 trait。
/// 类比 WPF `Application` + `StartupUri`。
///
/// 使用泛型类型状态模式 `RmlApplication<W>`：
/// - `RmlApplication<NoWindow>`：未设置主窗口，需用命令式 `run::<A>()`
/// - `RmlApplication<W: IWindow>`：已设置主窗口 `W`，用声明式 `run()`
pub struct RmlApplication<W = NoWindow> {
    _window: PhantomData<W>,
}

impl RmlApplication<NoWindow> {
    /// 创建应用启动器
    pub fn new() -> Self {
        Self { _window: PhantomData }
    }

    /// 声明式设置主窗口类型（WPF StartupUri 风格，内置方法）
    ///
    /// ```rust,ignore
    /// RmlApplication::new()
    ///     .main_window::<MainWindow>()
    ///     .run();
    /// ```
    pub fn main_window<NewW: IWindow + Default + 'static>(self) -> RmlApplication<NewW> {
        RmlApplication { _window: PhantomData }
    }

    /// 命令式启动（WPF OnStartup 重写风格）
    ///
    /// `A: IAppLifecycle` 负责窗口创建与生命周期。
    pub fn run<A>(self)
    where
        A: IAppLifecycle + Default + 'static,
    {
        gpui_platform::application()
            .with_assets(gpui_component_assets::Assets)
            .run(move |cx: &mut App| {
                let mut app = A::default();
                app.on_launch(cx);
            });
    }
}

impl<W: IWindow + Default + 'static> RmlApplication<W> {
    /// 启动应用并打开主窗口
    ///
    /// 框架自动：
    /// 1. 创建 `W::default()` 实例
    /// 2. 调用 `IWindow::open()` 打开主窗口
    pub fn run(self) {
        gpui_platform::application()
            .with_assets(gpui_component_assets::Assets)
            .run(move |cx: &mut App| {
                let mut window = W::default();
                window.open(cx);
            });
    }
}

impl Default for RmlApplication<NoWindow> {
    fn default() -> Self {
        Self::new()
    }
}

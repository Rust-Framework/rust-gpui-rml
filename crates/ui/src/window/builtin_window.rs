//! 内置窗口类型 —— 开箱即用的 `IWindow` 实现
//!
//! 提供 `Window`（基础窗口）和 `ModernWindow`（带 chrome 的现代窗口）。
//! 类比 WPF `Window` 类：可直接使用，也可作为更复杂窗口的基础。
//!
//! 用户创建带 RML 模板的窗口应使用 `#[window]` 宏。
//! 内置窗口适用于简单场景（占位窗口、启动画面、关于对话框等）。
//!
//! # 示例
//!
//! ```rust,ignore
//! use rml_ui::prelude::*;
//! use rml_app::RmlApplication;
//!
//! fn main() {
//!     RmlApplication::new()
//!         .main_window::<rml_ui::ModernWindow>()
//!         .run();
//! }
//! ```

use gpui::{
    App, AnyWindowHandle, AppContext, Context, IntoElement, ParentElement, Pixels, Render, div, px,
};

use rml_core::component::IComponent;
use rml_core::lifecycle::ILifecycle;
use rml_core::model::IModel;
use rml_core::view_model::IViewModel;
use rml_core::window::{IWindow, WindowChrome};

use super::modern_window::ModernWindowShell;

// ─── Window：基础窗口 ───────────────────────────────────────────

/// 基础窗口 —— 无装饰，仅提供窗口框架。
///
/// 类比 WPF `Window` 类：可直用，也可作为更复杂窗口的基础。
/// 窗口内容为占位符，用户创建带实际内容的窗口应使用 `#[window]` 宏。
pub struct Window {
    title: String,
    width: Pixels,
    height: Pixels,
    window_handle: Option<AnyWindowHandle>,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            title: String::from("RML Window"),
            width: px(800.),
            height: px(600.),
            window_handle: None,
        }
    }
}

impl Window {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置窗口标题
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// 设置窗口尺寸
    pub fn size(mut self, width: Pixels, height: Pixels) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

// 手动实现 trait 层级（ui crate 无 RML 构建过程，不能用 #[component] 宏）
impl IModel for Window {}
impl ILifecycle for Window {}
impl IViewModel for Window {}
impl IComponent for Window {
    fn rml_template() -> &'static str {
        ""
    }
    fn rml_tag() -> &'static str {
        "Window"
    }
}

impl Render for Window {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child("RML Window")
    }
}

impl IWindow for Window {
    fn title(&self) -> &str {
        &self.title
    }

    fn width(&self) -> Pixels {
        self.width
    }

    fn height(&self) -> Pixels {
        self.height
    }

    fn open(&mut self, cx: &mut App) {
        crate::init(cx);
        let options = self.window_options();
        let handle = cx
            .open_window(options, |window, cx| {
                let view = cx.new(|_| Self::default());
                cx.new(|cx| crate::Root::new(view, window, cx))
            })
            .expect("failed to open window");
        self.window_handle = Some(handle.into());
    }

    fn handle(&self) -> Option<AnyWindowHandle> {
        self.window_handle
    }

    fn set_handle(&mut self, handle: AnyWindowHandle) {
        self.window_handle = Some(handle);
    }
}

// ─── ModernWindow：现代窗口（带 chrome） ───────────────────────

/// 现代窗口 —— 使用 `ModernWindowShell` 提供 TitleBar/StatusBar 的现代窗口外观。
///
/// 类比 WPF 带 chrome 的 `Window`。窗口内容为占位符，
/// 用户创建带实际内容的窗口应使用 `#[window]` 宏。
pub struct ModernWindow {
    title: String,
    width: Pixels,
    height: Pixels,
    window_handle: Option<AnyWindowHandle>,
}

impl Default for ModernWindow {
    fn default() -> Self {
        Self {
            title: String::from("RML Application"),
            width: px(1024.),
            height: px(768.),
            window_handle: None,
        }
    }
}

impl ModernWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置窗口标题
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// 设置窗口尺寸
    pub fn size(mut self, width: Pixels, height: Pixels) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

impl IModel for ModernWindow {}
impl ILifecycle for ModernWindow {}
impl IViewModel for ModernWindow {}
impl IComponent for ModernWindow {
    fn rml_template() -> &'static str {
        ""
    }
    fn rml_tag() -> &'static str {
        "ModernWindow"
    }
}

impl Render for ModernWindow {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl IntoElement {
        ModernWindowShell::new().title(self.title.clone())
    }
}

impl IWindow for ModernWindow {
    fn title(&self) -> &str {
        &self.title
    }

    fn width(&self) -> Pixels {
        self.width
    }

    fn height(&self) -> Pixels {
        self.height
    }

    fn chrome(&self) -> WindowChrome {
        WindowChrome::Native
    }

    fn open(&mut self, cx: &mut App) {
        crate::init(cx);
        let options = self.window_options();
        let handle = cx
            .open_window(options, |window, cx| {
                let view = cx.new(|_| Self::default());
                cx.new(|cx| crate::Root::new(view, window, cx))
            })
            .expect("failed to open window");
        self.window_handle = Some(handle.into());
    }

    fn handle(&self) -> Option<AnyWindowHandle> {
        self.window_handle
    }

    fn set_handle(&mut self, handle: AnyWindowHandle) {
        self.window_handle = Some(handle);
    }
}

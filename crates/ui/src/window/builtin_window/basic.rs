//! 基础窗口 —— 无装饰，仅提供窗口框架
//!
//! 类比 WPF `Window` 类：可直用，也可作为更复杂窗口的基础。
//! 窗口内容为占位符，用户创建带实际内容的窗口应使用 `#[window]` 宏。

use gpui::{App, AnyWindowHandle, Context, IntoElement, ParentElement, Pixels, Render, div, px};

use rml_core::component::IComponent;
use rml_core::lifecycle::ILifecycle;
use rml_core::model::IModel;
use rml_core::view_model::IViewModel;
use rml_core::window::IWindow;

use super::super::ext::IWindowExt;

/// 基础窗口 —— 无装饰，仅提供窗口框架。
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
    fn template() -> &'static str {
        ""
    }
    fn tag() -> &'static str {
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
        self.open_rooted(cx);
    }

    fn handle(&self) -> Option<AnyWindowHandle> {
        self.window_handle
    }

    fn set_handle(&mut self, handle: AnyWindowHandle) {
        self.window_handle = Some(handle);
    }
}

//! 现代窗口 —— 使用 `ModernWindowShell` 提供 TitleBar/StatusBar 的现代窗口外观
//!
//! 类比 WPF 带 chrome 的 `Window`。窗口内容为占位符，
//! 用户创建带实际内容的窗口应使用 `#[window]` 宏。

use gpui::{App, AnyWindowHandle, Context, IntoElement, Pixels, Render, px};

use rml_core::component::IComponent;
use rml_core::lifecycle::ILifecycle;
use rml_core::model::IModel;
use rml_core::view_model::IViewModel;
use rml_core::window::{IWindow, WindowChrome};

use super::super::ext::IWindowExt;
use super::super::modern_window::ModernWindowShell;

/// 现代窗口 —— 使用 `ModernWindowShell` 提供 TitleBar/StatusBar 的现代窗口外观。
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
    fn template() -> &'static str {
        ""
    }
    fn tag() -> &'static str {
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

    // ModernWindowShell 自绘 TitleBar，需要透明标题栏模式
    // （appears_transparent: true + WindowDecorations::Client）。
    // 此前的 Native 是 bug，会导致 OS 原生标题栏覆盖自绘标题栏。
    fn chrome(&self) -> WindowChrome {
        WindowChrome::Transparent
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

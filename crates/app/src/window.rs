//! 窗口对象 —— WPF 风格的窗口管理
//!
//! 提供 `Window` / `ModernWindow` 对象，通过 `.open::<V>(cx)` 方法打开窗口。
//! 类比 WPF 的 `new Window().Show()` 模式。
//!
//! ## 典型用法
//!
//! ```rust,ignore
//! use rml_app::{IAppLifecycle, RmlApplication, ModernWindow};
//! use gpui::{App, px};
//!
//! struct MyApp;
//!
//! impl IAppLifecycle for MyApp {
//!     fn on_launch(&mut self, cx: &mut App) {
//!         // 创建 ModernWindow 对象并打开
//!         ModernWindow::new("My App", px(800.), px(600.))
//!             .open::<MyView>(cx);
//!     }
//! }
//!
//! fn main() {
//!     RmlApplication::new().run::<MyApp>();
//! }
//! ```
//!
//! ## Window vs ModernWindow
//!
//! - `Window`：使用系统原生标题栏，适用于简单窗口或自定义标题栏场景
//! - `ModernWindow`：透明标题栏，由 `TitleBar` 组件自绘，适用于 `<ModernWindow>` 根标签的 `.rml`

use gpui::{App, AppContext, Bounds, Pixels, Point, Render, Size, TitlebarOptions, WindowBounds, WindowOptions, px};
use rml_core::view::IRmlView;

/// 窗口装饰样式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowChrome {
    /// 系统原生标题栏（默认）
    Native,
    /// 透明标题栏 —— 由 `TitleBar` 组件自绘
    /// `appears_transparent = true` + 设置 traffic_light_position
    Transparent,
}

/// 窗口配置 —— WPF 风格的窗口对象
///
/// 通过 `Window::new(title, width, height).open::<V>(cx)` 打开窗口。
/// 使用系统原生标题栏。
///
/// 如需透明标题栏（配合 `<ModernWindow>` RML 标签），请使用 [`ModernWindow`]。
pub struct Window {
    title: gpui::SharedString,
    width: Pixels,
    height: Pixels,
    chrome: WindowChrome,
}

impl Window {
    /// 创建一个窗口对象
    ///
    /// ```rust,ignore
    /// Window::new("My App", px(800.), px(600.)).open::<MyView>(cx);
    /// ```
    pub fn new(title: impl Into<gpui::SharedString>, width: Pixels, height: Pixels) -> Self {
        Self {
            title: title.into(),
            width,
            height,
            chrome: WindowChrome::Native,
        }
    }

    /// 转换为 ModernWindow（透明标题栏）
    ///
    /// 等价于 `ModernWindow::new(title, width, height)`。
    pub fn into_modern(self) -> ModernWindow {
        ModernWindow(Window {
            chrome: WindowChrome::Transparent,
            ..self
        })
    }

    fn build_options(&self) -> WindowOptions {
        let titlebar = match self.chrome {
            WindowChrome::Native => TitlebarOptions {
                title: Some(self.title.clone()),
                appears_transparent: false,
                traffic_light_position: None,
            },
            WindowChrome::Transparent => TitlebarOptions {
                title: Some(self.title.clone()),
                appears_transparent: true,
                // 与 gpui-component::TitleBar::title_bar_options() 一致
                traffic_light_position: Some(Point::new(px(9.), px(9.))),
            },
        };

        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Default::default(),
                size: Size {
                    width: self.width,
                    height: self.height,
                },
            })),
            titlebar: Some(titlebar),
            ..Default::default()
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Feature `ui-components` 启用：用 Root 包裹业务 view
// ──────────────────────────────────────────────────────────────────────────

#[cfg(feature = "ui-components")]
impl Window {
    /// 打开窗口，以 `V` 为根视图
    ///
    /// 自动用 `rml_ui::Root` 包裹业务 view，从而支持 Dialog/Sheet/Notification 等浮层。
    ///
    /// ```rust,ignore
    /// Window::new("My App", px(800.), px(600.)).open::<MyView>(cx);
    /// ```
    pub fn open<V>(self, cx: &mut App) -> gpui::WindowHandle<rml_ui::Root>
    where
        V: IRmlView + Render + Default + 'static,
    {
        let options = self.build_options();
        cx.open_window(options, |window, cx| {
            let view = cx.new(|_cx| V::default());
            cx.new(|cx| rml_ui::Root::new(view, window, cx))
        })
        .expect("failed to open window")
    }
}

#[cfg(not(feature = "ui-components"))]
impl Window {
    /// 打开窗口，以 `V` 为根视图（退化路径：不包裹 Root）
    pub fn open<V>(self, cx: &mut App) -> gpui::WindowHandle<V>
    where
        V: IRmlView + Render + Default + 'static,
    {
        let options = self.build_options();
        cx.open_window(options, |_window, cx| cx.new(|_cx| V::default()))
            .expect("failed to open window")
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  ModernWindow —— 透明标题栏窗口
// ──────────────────────────────────────────────────────────────────────────

/// ModernWindow —— 透明标题栏的窗口对象
///
/// 与 [`Window`] 的区别：`titlebar.appears_transparent = true`，
/// 让 `TitleBar` 组件完全接管标题栏绘制（含窗口控制按钮）。
/// 适用于 `.rml` 根元素为 `<ModernWindow>` 的视图。
///
/// ```rust,ignore
/// ModernWindow::new("My App", px(800.), px(600.)).open::<MyView>(cx);
/// ```
pub struct ModernWindow(Window);

impl ModernWindow {
    /// 创建一个 ModernWindow 对象
    pub fn new(title: impl Into<gpui::SharedString>, width: Pixels, height: Pixels) -> Self {
        Self(Window {
            title: title.into(),
            width,
            height,
            chrome: WindowChrome::Transparent,
        })
    }

    /// 打开窗口（委托给内部 Window）
    #[cfg(feature = "ui-components")]
    pub fn open<V>(self, cx: &mut App) -> gpui::WindowHandle<rml_ui::Root>
    where
        V: IRmlView + Render + Default + 'static,
    {
        self.0.open::<V>(cx)
    }

    /// 打开窗口（退化路径：不包裹 Root）
    #[cfg(not(feature = "ui-components"))]
    pub fn open<V>(self, cx: &mut App) -> gpui::WindowHandle<V>
    where
        V: IRmlView + Render + Default + 'static,
    {
        self.0.open::<V>(cx)
    }
}

//! `IWindow` trait —— 窗口抽象接口
//!
//! 参考 WPF `Window` 类设计：
//! - 窗口 IS 组件（继承 `IComponent`，有模板和标签）
//! - 窗口有配置属性（`title` / `width` / `height` / `chrome`）
//! - 窗口自管理生命周期操作（`open` / `show` / `close` / `state`）
//!
//! 窗口操作（close/show/hide/activate/state）提供**默认实现**，基于 `handle()`
//! 调用 GPUI API。实现方只需提供 6 个核心方法即可获得完整窗口行为。
//! 由 `#[window]` 宏自动实现，也可手动 impl。

use gpui::{
    AnyWindowHandle, App, Bounds, Pixels, Point, Render, Size, TitlebarOptions, WindowBounds,
    WindowOptions, px,
};

use crate::component::IComponent;

/// 窗口标题栏样式（WPF: `Window.WindowStyle`）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowChrome {
    /// 系统原生标题栏
    Native,
    /// 透明标题栏（现代风格，由 `TitleBar` 组件自绘）
    #[default]
    Transparent,
}

/// 窗口状态（WPF: `WindowState`）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowState {
    #[default]
    Normal,
    Minimized,
    Maximized,
}

/// 标题栏窗口操作按钮可见性（minimize / maximize / close）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowControlButtons {
    pub minimize: bool,
    pub maximize: bool,
    pub close: bool,
}

impl Default for WindowControlButtons {
    fn default() -> Self {
        Self {
            minimize: true,
            maximize: true,
            close: true,
        }
    }
}

/// 窗口启动位置（WPF: `WindowStartupLocation`）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowStartupLocation {
    /// 使用 `left()` / `top()` 指定位置
    #[default]
    Manual,
    /// 屏幕居中（在 `open_rooted` 时根据显示器尺寸计算）
    CenterScreen,
}

/// 窗口抽象接口（WPF `Window` 类等价物）。
///
/// 窗口是一种特殊组件，可作为顶层 OS 窗口打开。
/// 窗口自管理其窗口句柄（`AnyWindowHandle`），无需扩展 trait。
///
/// # 自管理窗口操作
///
/// `close` / `show` / `hide` / `activate` / `state` 提供默认实现，
/// 基于 `handle()` 调用 GPUI API。实现方只需覆盖核心方法：
/// - 配置：`title()` / `width()` / `height()`
/// - 句柄：`handle()` / `set_handle()`
/// - 打开：`open()`
///
/// 通过 `#[window]` 宏自动实现，或手动 impl。
///
/// # 示例
///
/// ```rust,ignore
/// #[window]
/// #[derive(Default)]
/// pub struct MainWindow {
///     count: i32,
/// }
///
/// fn main() {
///     RmlApplication::new()
///         .main_window::<MainWindow>()
///         .run();
/// }
/// ```
pub trait IWindow: IComponent + Default + Render {
    // ── 必需：配置属性（WPF: Window.Title / Width / Height）──

    /// 窗口标题
    fn title(&self) -> &str;

    /// 窗口宽度
    fn width(&self) -> Pixels;

    /// 窗口高度
    fn height(&self) -> Pixels;

    // ── 必需：句柄管理 ──

    /// 获取窗口句柄（未打开时返回 `None`）
    fn handle(&self) -> Option<AnyWindowHandle>;

    /// 设置窗口句柄（由 `open()` 内部调用）
    fn set_handle(&mut self, handle: AnyWindowHandle);

    // ── 必需：打开窗口（WPF: `Window.Show()`）──
    //
    // 创建 OS 窗口并显示。窗口句柄存储在实例内部。
    // 默认实现见 rml_ui 的 `IWindowExt::open_rooted`（含 init + Root 包裹），
    // rml_core 不依赖 rml_ui 故无法在此提供默认实现。
    fn open(&mut self, cx: &mut App);

    // ── 默认：窗口装饰（WPF: Window.WindowStyle）──

    /// 标题栏样式（默认透明/现代风格）
    fn chrome(&self) -> WindowChrome {
        WindowChrome::Transparent
    }

    /// 窗口左边距（`Manual` 启动位置时有效）
    fn left(&self) -> Option<Pixels> {
        None
    }

    /// 窗口顶边距（`Manual` 启动位置时有效）
    fn top(&self) -> Option<Pixels> {
        None
    }

    /// 窗口启动位置
    fn startup_location(&self) -> WindowStartupLocation {
        WindowStartupLocation::Manual
    }

    /// 最小窗口尺寸
    fn min_size(&self) -> Option<Size<Pixels>> {
        None
    }

    /// 是否允许用户调整窗口大小
    fn resizable(&self) -> bool {
        true
    }

    /// 标题栏窗口操作按钮可见性（透明标题栏模式下由自绘 `WindowControls` 生效）
    fn window_controls(&self) -> WindowControlButtons {
        WindowControlButtons::default()
    }

    // ── 默认：窗口选项构建 ──

    /// 从配置构建 GPUI `WindowOptions`
    ///
    /// - `WindowChrome::Native`：使用 OS 原生标题栏（`appears_transparent: false`，
    ///   `window_decorations` 留默认 `Server`），由 OS 绘制 min/max/close 按钮
    /// - `WindowChrome::Transparent`：使用 `TitleBar` 组件自绘标题栏
    ///   （`appears_transparent: true` + `window_decorations: Client`），
    ///   在 Windows/Linux 上需要 `Client` 装饰才能让 `TitleBar::WindowControls`
    ///   的 `window_control_area` hit-test 区域生效，并避免 OS 标题栏覆盖自绘标题栏
    fn window_options(&self) -> WindowOptions {
        let (titlebar, decorations) = match self.chrome() {
            WindowChrome::Native => (
                TitlebarOptions {
                    title: Some(self.title().into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                },
                None,
            ),
            WindowChrome::Transparent => (
                TitlebarOptions {
                    title: Some(self.title().into()),
                    appears_transparent: true,
                    traffic_light_position: Some(Point::new(px(9.), px(9.))),
                },
                // 自绘标题栏模式：必须 Client 装饰才能让 WindowControls 工作
                Some(gpui::WindowDecorations::Client),
            ),
        };

        let origin = match self.startup_location() {
            WindowStartupLocation::Manual => Point::new(
                self.left().unwrap_or_default(),
                self.top().unwrap_or_default(),
            ),
            WindowStartupLocation::CenterScreen => Point::default(),
        };

        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin,
                size: Size {
                    width: self.width(),
                    height: self.height(),
                },
            })),
            titlebar: Some(titlebar),
            window_decorations: decorations,
            is_resizable: self.resizable(),
            window_min_size: self.min_size(),
            ..Default::default()
        }
    }

    /// 根据启动位置修正窗口 bounds（`CenterScreen` 需在打开时调用）
    fn resolve_window_bounds(&self, cx: &App) -> WindowBounds {
        let size = Size {
            width: self.width(),
            height: self.height(),
        };
        match self.startup_location() {
            WindowStartupLocation::CenterScreen => {
                if let Some(display) = cx.primary_display() {
                    let bounds = display.bounds();
                    let origin = Point::new(
                        bounds.origin.x + (bounds.size.width - size.width) / 2.,
                        bounds.origin.y + (bounds.size.height - size.height) / 2.,
                    );
                    return WindowBounds::Windowed(Bounds { origin, size });
                }
                WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size,
                })
            }
            WindowStartupLocation::Manual => WindowBounds::Windowed(Bounds {
                origin: Point::new(
                    self.left().unwrap_or_default(),
                    self.top().unwrap_or_default(),
                ),
                size,
            }),
        }
    }

    /// 构建带显示器上下文的 `WindowOptions`（处理 `CenterScreen`）
    fn window_options_for(&self, cx: &App) -> WindowOptions {
        let mut options = self.window_options();
        options.window_bounds = Some(self.resolve_window_bounds(cx));
        options
    }

    // ── 默认：窗口操作（基于 handle 自管理，WPF: Window.Show/Close/Activate）──

    /// 关闭窗口（WPF: `Window.Close()`）
    ///
    /// 默认实现通过 `handle()` 调用 GPUI `window.remove_window()`。
    fn close(&mut self, cx: &mut App) {
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| {
                window.remove_window();
            });
        }
    }

    /// 显示窗口（若已隐藏）
    ///
    /// 默认实现通过 `handle()` 调用 GPUI `window.activate_window()`。
    fn show(&mut self, cx: &mut App) {
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| {
                window.activate_window();
            });
        }
    }

    /// 隐藏窗口（WPF: `Window.Hide()`）
    ///
    /// GPUI 不支持单窗口隐藏，默认实现使用 `minimize_window()` 作为最接近的替代。
    fn hide(&mut self, cx: &mut App) {
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| {
                window.minimize_window();
            });
        }
    }

    /// 激活窗口（置于前台）
    ///
    /// 默认实现通过 `handle()` 调用 GPUI `window.activate_window()`。
    fn activate(&mut self, cx: &mut App) {
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| {
                window.activate_window();
            });
        }
    }

    // ── 默认：状态查询（WPF: Window.WindowState）──

    /// 获取窗口状态
    ///
    /// 默认实现通过 `handle()` 查询 GPUI `window.is_maximized()`。
    /// 注意：GPUI 仅支持查询 `is_maximized()`，无法查询最小化状态。
    /// `WindowState::Minimized` 需由实现方覆盖此方法自行追踪。
    fn state(&self, cx: &mut App) -> WindowState {
        if let Some(handle) = self.handle() {
            if let Ok(maximized) = handle.update(cx, |_view, window, _cx| {
                window.is_maximized()
            }) {
                if maximized {
                    return WindowState::Maximized;
                }
            }
        }
        WindowState::Normal
    }

    // ── 默认：状态操作（WPF: Window.WindowState setter）──

    /// 设置窗口状态（WPF: `Window.WindowState = ...`）
    ///
    /// 基于 GPUI `zoom_window` toggle 语义：
    /// - `Minimized` → `minimize_window()`
    /// - `Maximized` → 仅当未最大化时调用 `zoom_window()`（避免重复 toggle 还原）
    /// - `Normal` → 仅当已最大化时调用 `zoom_window()`（toggle 还原）
    fn set_state(&mut self, state: WindowState, cx: &mut App) {
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| match state {
                WindowState::Minimized => window.minimize_window(),
                WindowState::Maximized => {
                    if !window.is_maximized() {
                        window.zoom_window();
                    }
                }
                WindowState::Normal => {
                    if window.is_maximized() {
                        window.zoom_window();
                    }
                }
            });
        }
    }

    /// 最大化窗口
    fn maximize(&mut self, cx: &mut App) {
        self.set_state(WindowState::Maximized, cx);
    }

    /// 最小化窗口
    fn minimize(&mut self, cx: &mut App) {
        self.set_state(WindowState::Minimized, cx);
    }

    /// 还原窗口（从最大化/最小化恢复）
    fn restore(&mut self, cx: &mut App) {
        self.set_state(WindowState::Normal, cx);
    }
}

//! `RmlApplication` —— 应用启动器
//!
//! 封装 GPUI 的 `Application` + 单窗口创建，提供 `RmlApplication::new().run::<RootView>()` API。
//! 详见文档 §1.3.6 入口编写。
//!
//! ## Feature `ui-components`（默认开启）
//!
//! 启用时：在 `Application::run` 启动回调中调用 `rml_ui::init(cx)`，并将窗口根 view 替换为
//! `rml_ui::Root` 包裹业务 view（`Entity<R>`），从而启用 Dialog/Sheet/Notification/Tooltip
//! 等浮层组件支持。
//!
//! 关闭时：业务 view 直接作为窗口根 view，不引入 gpui-component 依赖。

use gpui::{
    App, AppContext, Bounds, Entity, Pixels, Render, Size, TitlebarOptions, WindowBounds,
    WindowOptions, px,
};
#[cfg(feature = "ui-components")]
use gpui::Window;
use rml_core::view::IRmlView;

/// RML 应用启动器
///
/// ```rust
/// use rml_app::RmlApplication;
///
/// fn main() {
///     RmlApplication::new()
///         .title("My App")
///         .size(px(800.), px(600.))
///         .run::<MyView>();
/// }
/// ```
pub struct RmlApplication {
    title: gpui::SharedString,
    width: Pixels,
    height: Pixels,
}

impl RmlApplication {
    pub fn new() -> Self {
        Self {
            title: "RML App".into(),
            width: px(800.),
            height: px(600.),
        }
    }

    /// 设置窗口标题
    pub fn title(mut self, t: impl Into<gpui::SharedString>) -> Self {
        self.title = t.into();
        self
    }

    /// 设置窗口尺寸
    pub fn size(mut self, w: Pixels, h: Pixels) -> Self {
        self.width = w;
        self.height = h;
        self
    }

    /// 启动应用，以 `R` 为根视图。
    ///
    /// `R` 必须实现 `IRmlView`（marker）、`Render`（由 `#[view]` + build.rs 生成）、
    /// `Default`（用于构造初始实例）。
    ///
    /// 启用 feature `ui-components` 时，`R` 会作为子 view 嵌入 `rml_ui::Root`，
    /// 后者负责 Dialog/Sheet/Notification 等浮层管理。
    pub fn run<R>(self)
    where
        R: IRmlView + Render + Default + 'static,
    {
        let title = self.title;
        let size = Size {
            width: self.width,
            height: self.height,
        };

        gpui_platform::application().run(move |cx: &mut App| {
            // 初始化 gpui-component 全局状态：theme / global_state / root / dialog / ...
            // 必须在打开窗口前完成
            #[cfg(feature = "ui-components")]
            rml_ui::init(cx);

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size,
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some(title.clone()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                ..Default::default()
            };

            cx.open_window(options, open_window_root::<R>)
                .expect("failed to open window");
        });
    }
}

/// 窗口构建闭包：根据 feature 决定返回 `Entity<Root>` 或 `Entity<R>`。
///
/// 独立为函数便于在 cfg 分支中分别实现，避免闭包内联时类型推断混淆。
#[cfg(feature = "ui-components")]
fn open_window_root<R>(window: &mut Window, cx: &mut App) -> Entity<rml_ui::Root>
where
    R: IRmlView + Render + Default + 'static,
{
    // 1. 构造业务 view
    let view = cx.new(|_cx| R::default());
    // 2. 用 Root 包裹，从而获得 Dialog/Sheet/Notification 等浮层支持
    // Root::new 第三个参数为 &mut Context<Root>，由 cx.new::<Root> 提供
    cx.new(|cx| rml_ui::Root::new(view, window, cx))
}

#[cfg(not(feature = "ui-components"))]
fn open_window_root<R>(_window: &mut gpui::Window, cx: &mut App) -> Entity<R>
where
    R: IRmlView + Render + Default + 'static,
{
    cx.new(|_cx| R::default())
}

impl Default for RmlApplication {
    fn default() -> Self {
        Self::new()
    }
}

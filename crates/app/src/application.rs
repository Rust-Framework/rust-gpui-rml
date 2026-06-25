//! `RmlApplication` —— 应用启动器
//!
//! 封装 GPUI 的 `Application` + 单窗口创建，提供 `RmlApplication::new().run::<RootView>()` API。
//! 详见文档 §1.3.6 入口编写。

use gpui::{
    App, Application, Bounds, Entity, IntoElement, Pixels, Render, Size, TitlebarOptions,
    WindowBounds, WindowOptions,
};
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
    pub fn run<R>(self)
    where
        R: IRmlView + Render + Default + 'static,
    {
        let title = self.title;
        let size = Size {
            width: self.width,
            height: self.height,
        };

        Application::new().run(move |cx: &mut App| {
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

            cx.open_window(options, |_window, cx| -> Entity<R> {
                cx.new(|_cx| R::default())
            })
            .expect("failed to open window");
        });
    }
}

impl Default for RmlApplication {
    fn default() -> Self {
        Self::new()
    }
}

fn px(f: f32) -> Pixels {
    Pixels(f)
}

//! `RmlApplication` —— 应用启动器

use std::marker::PhantomData;

use gpui::App;
use rml_core::i18n::ensure_i18n;
use rml_core::theme::ensure_theme;
use rml_core::window::IWindow;

use crate::lifecycle::{IAppLifecycle, NoLifecycle};

/// 标记：未设置主窗口
pub struct NoWindow;

/// RML 应用启动器
pub struct RmlApplication<W = NoWindow, L = NoLifecycle> {
    _window: PhantomData<W>,
    _lifecycle: PhantomData<L>,
}

impl RmlApplication<NoWindow, NoLifecycle> {
    pub fn new() -> Self {
        Self {
            _window: PhantomData,
            _lifecycle: PhantomData,
        }
    }

    /// 声明式：设置主窗口类型
    pub fn main_window<NewW: IWindow + Default + 'static>(
        self,
    ) -> RmlApplication<NewW, NoLifecycle> {
        RmlApplication {
            _window: PhantomData,
            _lifecycle: PhantomData,
        }
    }

    /// 命令式启动：`on_launch` 完全控制窗口创建（登录窗 → 主窗口等）
    pub fn run<A>(self)
    where
        A: IAppLifecycle + 'static,
    {
        gpui_platform::application()
            .with_assets(gpui_component_assets::Assets)
            .run(move |cx: &mut App| {
                ensure_i18n(cx);
                ensure_theme(cx);
                let mut app = A::default();
                app.on_launch(cx);
            });
    }
}

impl<W: IWindow + Default + 'static> RmlApplication<W, NoLifecycle> {
    /// 声明式：在主窗口 `open()` 之前执行启动逻辑
    pub fn lifecycle<NewL: IAppLifecycle + 'static>(
        self,
    ) -> RmlApplication<W, NewL> {
        RmlApplication {
            _window: PhantomData,
            _lifecycle: PhantomData,
        }
    }
}

impl<W: IWindow + Default + 'static, L: IAppLifecycle + 'static> RmlApplication<W, L> {
    /// 声明式启动：先 `L::on_launch`，再打开主窗口 `W`
    pub fn run(self) {
        gpui_platform::application()
            .with_assets(gpui_component_assets::Assets)
            .run(move |cx: &mut App| {
                ensure_i18n(cx);
                ensure_theme(cx);
                let mut hooks = L::default();
                hooks.on_launch(cx);
                let mut window = W::default();
                window.open(cx);
            });
    }
}

impl Default for RmlApplication<NoWindow, NoLifecycle> {
    fn default() -> Self {
        Self::new()
    }
}

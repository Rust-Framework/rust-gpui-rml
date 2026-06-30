//! `RmlApplication` —— 应用启动器
//!
//! 类比 WPF / .NET 的 `Program.cs`：通过 builder 链式配置应用级资源与主窗口，
//! 最后调用 `.run::<L>()` 启动。框架自动管理主窗口的创建与生命周期。
//!
//! ```rust,ignore
//! rml::embed_assets!();
//!
//! fn main() {
//!     rml_app::RmlApplication::new()
//!         .assets(RML_ASSETS)
//!         .main_window::<MainWindow>()
//!         .run::<app::Startup>();
//! }
//! ```

use std::marker::PhantomData;

use gpui::{App, Window};
use rml_core::i18n::ensure_i18n;
use rml_core::theme::ensure_theme;
use rml_core::window::IWindow;

use crate::lifecycle::IAppLifecycle;

/// 标记：未设置主窗口
pub struct NoWindow;

/// 嵌入资源表类型（由 `rml::embed_assets!()` 宏在用户 crate 根生成）
pub type AssetsTable = &'static [(&'static str, &'static [u8])];

/// RML 应用启动器
///
/// - `RmlApplication<NoWindow>`：命令式入口，`run::<A>()` 由 `A::on_launch` 全权控制
/// - `RmlApplication<W>`：声明式入口，`run::<L>()` 自动打开 `W` 并驱动 `L` 生命周期
pub struct RmlApplication<W = NoWindow> {
    assets: Option<AssetsTable>,
    _window: PhantomData<W>,
}

impl RmlApplication<NoWindow> {
    pub fn new() -> Self {
        Self {
            assets: None,
            _window: PhantomData,
        }
    }

    /// 注册嵌入资源表（由 `rml::embed_assets!()` 在编译期生成）。
    pub fn assets(mut self, assets: AssetsTable) -> Self {
        self.assets = Some(assets);
        self
    }

    /// 声明主窗口类型，切换到声明式入口。
    pub fn main_window<W: IWindow + Default + 'static>(self) -> RmlApplication<W> {
        RmlApplication {
            assets: self.assets,
            _window: PhantomData,
        }
    }

    /// 命令式启动：`on_launch` 完全控制窗口创建（无主窗口自动管理）。
    pub fn run<A: IAppLifecycle + 'static>(self) {
        if let Some(assets) = self.assets {
            rml_core::assets::init(assets);
        }
        gpui_platform::application()
            .with_assets(gpui_component_assets::Assets)
            .run(move |cx: &mut App| {
                ensure_i18n(cx);
                ensure_theme(cx);
                A::default().on_launch(cx);
            });
    }
}

impl<W: IWindow + Default + 'static> RmlApplication<W> {
    /// 声明式启动：`L::on_launch` → 打开主窗口 `W` → `L::on_main_window_ready`。
    pub fn run<L: IAppLifecycle + 'static>(self) {
        if let Some(assets) = self.assets {
            rml_core::assets::init(assets);
        }
        gpui_platform::application()
            .with_assets(gpui_component_assets::Assets)
            .run(move |cx: &mut App| {
                ensure_i18n(cx);
                ensure_theme(cx);
                let mut hooks = L::default();
                hooks.on_launch(cx);
                let mut window = W::default();
                window.open(cx);
                if let Some(handle) = window.handle() {
                    let _ = handle.update(cx, |_, window: &mut Window, cx| {
                        hooks.on_main_window_ready(window, cx);
                    });
                }
            });
    }
}

impl Default for RmlApplication<NoWindow> {
    fn default() -> Self {
        Self::new()
    }
}

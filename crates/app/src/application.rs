//! `RmlApplication` —— 应用启动器
//!
//! 类比 WPF / .NET 的 `Program.cs`：通过 builder 链式配置应用级资源与主窗口,
//! 最后调用 `.run::<L>()` 启动。框架自动管理主窗口的创建与生命周期。
//!
//! 资源注册由 build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动完成
//! （通过 `#[rml::main]` 属性宏注入 `rml::embed_assets!()` 触发 include!()）,
//! 因此 main.rs 无需调用 `.assets(...)`。
//!
//! ```rust,ignore
//! #[rml::main]
//! fn main() {
//!     rml_app::RmlApplication::new()
//!         .main_window::<MainWindow>()
//!         .run::<app::Startup>();
//! }
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use gpui::{px, App};
use rml_core::context::ensure_service_collection;
use rml_core::context::IAppContext;
use rml_core::i18n::ensure_i18n;
use rml_core::theme::ensure_theme;
use rml_core::window::IWindow;

use crate::lifecycle::IAppLifecycle;

fn bootstrap_runtime(cx: &mut App) {
    // 初始化 IAppContext 的 ServiceCollection（IServiceProvider 风格统一服务访问）
    ensure_service_collection(cx);
    // 注册 ContributionRegistry 为单例服务（替代原 OnceLock 静态存储）
    cx.set_service(Arc::new(crate::contribution::ContributionRegistry::new()));

    ensure_i18n(cx);
    ensure_theme(cx);
    gpui_component::init(cx);
    gpui_component::Theme::global_mut(cx).font_size = px(14.);
    // 贡献注册由 host 在 on_loaded 中手动触发（registry.add(host) → bootstrap_host_contributions）
}

/// 标记：未设置主窗口
pub struct NoWindow;

/// RML 应用启动器
///
/// - `RmlApplication<NoWindow>`：命令式入口,`run::<A>()` 由 `A::on_launch` 全权控制
/// - `RmlApplication<W>`：声明式入口,`run::<L>()` 自动打开 `W` 并驱动 `L` 生命周期
pub struct RmlApplication<W = NoWindow> {
    _window: PhantomData<W>,
}

impl RmlApplication<NoWindow> {
    pub fn new() -> Self {
        Self {
            _window: PhantomData,
        }
    }

    /// 声明主窗口类型,切换到声明式入口。
    pub fn main_window<W: IWindow + Default + 'static>(self) -> RmlApplication<W> {
        RmlApplication {
            _window: PhantomData,
        }
    }

    /// 命令式启动：`on_launch` 完全控制窗口创建（无主窗口自动管理）。
    ///
    /// 资源已由 build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动注册,
    /// 此处无需任何 init 调用。
    pub fn run<A: IAppLifecycle + 'static>(self) {
        gpui_platform::application()
            .with_assets(gpui_component_assets::Assets)
            .run(move |cx: &mut App| {
                bootstrap_runtime(cx);
                A::default().on_launch(cx);
            });
    }
}

impl<W: IWindow + Default + 'static> RmlApplication<W> {
    /// 声明式启动：`L::on_launch` → 打开主窗口 `W`。
    ///
    /// 资源已由 build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动注册,
    /// 此处无需任何 init 调用。
    pub fn run<L: IAppLifecycle + 'static>(self) {
        gpui_platform::application()
            .with_assets(gpui_component_assets::Assets)
            .run(move |cx: &mut App| {
                bootstrap_runtime(cx);
                L::default().on_launch(cx);
                W::default().open(cx);
            });
    }
}

impl Default for RmlApplication<NoWindow> {
    fn default() -> Self {
        Self::new()
    }
}

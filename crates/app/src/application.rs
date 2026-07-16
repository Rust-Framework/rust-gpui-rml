//! `RmlApplication` —— 应用启动器
//!
//! 类比 WPF / .NET 的 `Program.cs`：通过 builder 链式配置应用级资源与主窗口,
//! 最后调用 `.run::<L>()` 启动。框架自动管理主窗口的创建与生命周期。
//!
//! 资源注册由 build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动完成
//! （通过 `#[rml::main]` 属性宏注入 `rml::embed_assets!()` 触发 include!()）,
//! 因此 main.rs 无需调用 `.assets(...)`。
//!
//! DI 容器由产品层（如 Studio）在 `ILifecycle::on_loaded` 中自行构建并经
//! `cx.use_provider()` 注入。框架本身不绑定特定 DI 实现。
//!
//! ```rust,ignore
//! #[rml::main]
//! fn main() {
//!     rml_app::RmlApplication::new()
//!         .main_window::<MainWindow>()
//!         .run::<app::Startup>();
//! }
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use gpui::{px, App};
use rml_core::context::ensure_service_provider;
use rml_core::context::{IAppContext, IServiceProvider};
use rml_core::i18n::ensure_i18n;
use rml_core::theme::ensure_theme;
use rml_core::window::IWindow;

use crate::lifecycle::IAppLifecycle;

fn bootstrap_runtime(cx: &mut App) {
    // 初始化 IAppContext 的 ServiceProviderSlot（IServiceProvider 风格统一服务访问）
    ensure_service_provider(cx);
    // 注册 ContributionRegistry 为单例服务（替代原 OnceLock 静态存储）
    cx.set_service(Arc::new(crate::contribution::ContributionRegistry::new()));

    ensure_i18n(cx);
    ensure_theme(cx);
    gpui_component::init(cx);
    gpui_component::Theme::global_mut(cx).font_size = px(14.);
    // 贡献注册由 host 在 on_loaded 中手动触发（cx.register_host(id, host) → bootstrap_host_contributions）
}

/// 标记：未设置主窗口
pub struct NoWindow;

/// RML 应用启动器
///
/// - `RmlApplication<NoWindow>`：命令式入口,`run::<A>()` 由 `A::on_launch` 全权控制
/// - `RmlApplication<W>`：声明式入口,`run::<L>()` 自动打开 `W` 并驱动 `L` 生命周期
///
/// `properties` 提供通用 key-value 存储（`get/set`），可供产品层在 `run` 之前
/// 注入 `Arc<dyn IServiceProvider>`，`run` 时取出并经 `cx.use_provider` 注入为正式后端。
/// 当前 Studio 经 `MainWindow::on_loaded` → `cx.use_provider` 二阶段注入,不使用此机制。
///
/// 注意：`RmlApplication` 是单线程 builder（仅 `main()` 中使用，事件循环前消费），
/// 故 `properties` 不要求 `Send + Sync`，可存储任意 `'static` 类型（含 `Arc<dyn Trait>`）。
pub struct RmlApplication<W = NoWindow> {
    _window: PhantomData<W>,
    properties: HashMap<TypeId, Box<dyn Any>>,
}

impl<W> RmlApplication<W> {
    /// 存入属性（按类型键）。DI 适配层用此注入 `Arc<dyn IServiceProvider>`。
    pub fn set<T: 'static>(&mut self, value: T) {
        self.properties.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// 按类型键读取属性引用。
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.properties
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    /// 取出（移动）属性。`run` 阶段用于取出 provider 注入到 `IAppContext`。
    fn take<T: 'static>(&mut self) -> Option<T> {
        self.properties
            .remove(&TypeId::of::<T>())
            .and_then(|b| b.downcast::<T>().ok())
            .map(|b| *b)
    }
}

impl RmlApplication<NoWindow> {
    pub fn new() -> Self {
        Self {
            _window: PhantomData,
            properties: HashMap::new(),
        }
    }

    /// 声明主窗口类型,切换到声明式入口。
    pub fn main_window<W: IWindow + Default + 'static>(mut self) -> RmlApplication<W> {
        // 转移 properties（configure 阶段可能已注入 provider）
        let properties = std::mem::take(&mut self.properties);
        RmlApplication {
            _window: PhantomData,
            properties,
        }
    }

    /// 命令式启动：`on_launch` 完全控制窗口创建（无主窗口自动管理）。
    ///
    /// 若 `configure` 阶段注入了 `Arc<dyn IServiceProvider>`,此处取出并经
    /// `cx.use_provider` 注入为正式后端。
    ///
    /// 资源已由 build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动注册,
    /// 此处无需任何 init 调用。
    pub fn run<A: IAppLifecycle + 'static>(mut self) {
        let provider = self.take::<Arc<dyn IServiceProvider + Send + Sync>>();
        gpui_platform::application()
            .with_assets(crate::assets::CompositeAssets)
            .run(move |cx: &mut App| {
                bootstrap_runtime(cx);
                if let Some(p) = provider {
                    cx.use_provider(p);
                }
                A::default().on_launch(cx);
            });
    }
}

impl<W: IWindow + Default + 'static> RmlApplication<W> {
    /// 声明式启动：`L::on_launch` → 打开主窗口 `W`。
    ///
    /// 若 `configure` 阶段注入了 `Arc<dyn IServiceProvider + Send + Sync>`,此处取出并经
    /// `cx.use_provider` 注入为正式后端。
    ///
    /// 资源已由 build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动注册,
    /// 此处无需任何 init 调用。
    pub fn run<L: IAppLifecycle + 'static>(mut self) {
        let provider = self.take::<Arc<dyn IServiceProvider + Send + Sync>>();
        gpui_platform::application()
            .with_assets(crate::assets::CompositeAssets)
            .run(move |cx: &mut App| {
                bootstrap_runtime(cx);
                if let Some(p) = provider {
                    cx.use_provider(p);
                }
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

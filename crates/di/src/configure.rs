//! RmlApplicationExt —— configure 模式（闭包驱动）
//!
//! 对标 ASP.NET Core `ConfigureServices`。
//! `RmlApplication::new().configure(|s| { ... }).run::<L>()` 链式 API：
//! - `configure` 接收 `FnOnce(&mut ServiceCollection)` 闭包，用户直接操作 ServiceCollection
//! - 内部创建 collection → 应用自动注册 → 调用闭包 → build provider → `set` 进 RmlApplication
//! - `run` 时取出 provider，经 `cx.use_provider` 注入
//!
//! 封装式用法：将配置逻辑提取到 `Configure::build` 静态函数，经 `.configure(Configure::build)` 调用，
//! 职责清晰，框架封装创建/自动注册/build 等样板。

use std::sync::Arc;

use rml_app::application::RmlApplication;
use rml_core::context::IServiceProvider;

use crate::auto_register::apply_auto_registrations;
use crate::collection::ServiceCollection;

/// RmlApplication 扩展 —— `configure` 链式 API。
///
/// `configure(|s| { ... })` 内部：
/// 1. 创建 `ServiceCollection`
/// 2. `apply_auto_registrations` —— 应用 `ctor` 自动注册
/// 3. 调用闭包 —— 用户配置
/// 4. `collection.build()` → `Arc<dyn IServiceProvider + Send + Sync>`
/// 5. `self.set(provider)` —— 存入 RmlApplication properties
///
/// `run::<L>()` 时取出 provider，经 `cx.use_provider` 注入。
///
/// # 用法
///
/// 简单场景直接内联闭包：
/// ```rust,ignore
/// RmlApplication::new()
///     .main_window::<MainWindow>()
///     .configure(|s| {
///         s.add_singleton::<dyn IFoo>(|_| Arc::new(FooImpl));
///     })
///     .run::<Startup>()
/// ```
///
/// 复杂场景提取到独立函数（职责清晰）：
/// ```rust,ignore
/// RmlApplication::new()
///     .main_window::<MainWindow>()
///     .configure(AppConfig::build)
///     .run::<Startup>()
/// ```
pub trait RmlApplicationExt<W>: Sized {
    fn configure<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut ServiceCollection);
}

impl<W> RmlApplicationExt<W> for RmlApplication<W> {
    fn configure<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut ServiceCollection),
    {
        let mut collection = ServiceCollection::new();
        apply_auto_registrations(&mut collection);
        f(&mut collection);
        let provider: Arc<dyn IServiceProvider + Send + Sync> = collection.build();
        self.set::<Arc<dyn IServiceProvider + Send + Sync>>(provider);
        self
    }
}

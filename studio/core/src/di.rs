//! rust-dix → IServiceProvider 桥接 —— Studio 产品层 DI 集成
//!
//! Studio 选择 rust-dix 作为 DI 容器，通过本模块桥接到 RML 框架的
//! `IServiceProvider` trait。桥接是薄包装：`get_service_any` 直接委托
//! rust-dix 的 `get_by_type_id`（O(1) TypeId 直查）。
//!
//! ## 自动注册（`#[inject]`）
//!
//! 使用 `#[rust_dix::inject]` 标记服务，自动注册到容器：
//! ```rust,ignore
//! use studio_core::di::inject;
//!
//! #[inject]
//! struct MyService;
//!
//! #[inject]
//! impl IMyTrait for MyService { ... }
//! ```
//! `ServiceCollection::from_injected()` 自动收集所有 `#[inject]` 标记的服务。
//!
//! ## 手动注册（trait object）
//!
//! rust-dix 原生支持 `?Sized` trait object 注册/查询，无需 `ServiceSlot` 桥接。
//! 由于 rust-dix 的 `singleton` / `keyed_singleton` 是 builder 链式 API（消费 `self`），
//! 而 `auto_register` 闭包接收 `&mut ServiceCollection`，本模块提供 `ServiceCollectionExt`
//! 适配器（`&mut self` 转发原生 API）：
//! ```rust,ignore
//! use std::sync::Arc;
//! use studio_core::di::{ServiceCollection, ServiceCollectionExt};
//!
//! s.add_singleton::<dyn IWorkbenchProvider>(|_| {
//!     Arc::new(EditorProvider) as Arc<dyn IWorkbenchProvider>
//! });
//! s.add_keyed_singleton::<dyn IWorkbenchProvider>("file", |_| {
//!     Arc::new(EditorProvider) as Arc<dyn IWorkbenchProvider>
//! });
//! ```
//! 查询时经 `cx.get_service::<dyn IWorkbenchProvider>()` 或
//! `cx.get_keyed_service::<dyn IWorkbenchProvider>("file")` 直接解析（`IServiceProvider::get_service<T: ?Sized>`）。
//!
//! ## 子主容器桥接（`DixServiceWrapper`）
//!
//! 插件子容器经 `ServiceProviderWrapper` 桥接主容器，child-first 解析：
//! ```rust,ignore
//! let wrapper = ServiceProviderWrapper::new(child_provider, root_provider);
//! let bridge = DixServiceWrapper::new(wrapper);
//! cx.set_provider(bridge);  // 覆盖式注入为正式 provider
//! ```
//!
//! `IAppContext::set_provider` 为覆盖语义，分层逻辑由 rust-dix `ServiceProviderWrapper`
//! 原生表达（child-first + root fallback），RML 框架不维护多 provider 串联链。

use std::any::{Any, TypeId};
use std::sync::{Arc, Mutex, OnceLock};

use rml_core::context::IServiceProvider;
use rust_dix::entry::IServiceResolver;
use rust_dix::ServiceProvider as RdiProvider;
use rust_dix::ServiceProviderWrapper as RdiWrapper;

// ── rust-dix 宏 re-export（供 Studio 全局使用）──
pub use rust_dix::{inject, module, register, Inject};

/// Studio ServiceCollection —— 直接 re-export rust-dix 原生类型。
///
/// `new()` 等价 `rust_dix::ServiceCollection::new()`，
/// `from_injected()` 收集所有 `#[inject]` 标记的服务。
pub use rust_dix::ServiceCollection;

// ──────────────────────────────────────────────────────────────────────────
//  ServiceCollectionExt —— &mut self 适配器（转发 rust-dix 原生 builder API）
// ──────────────────────────────────────────────────────────────────────────

/// `ServiceCollection` 扩展 —— `&mut self` 转发 rust-dix 原生 `singleton` / `keyed_singleton`。
///
/// rust-dix 的 builder 方法消费 `self` 返回 `Self`，不便在 `&mut ServiceCollection`
/// 闭包中使用。本 trait 提供 `&mut self` 语义的便捷方法，内部用 `std::mem::replace`
/// 取出再写回，直接转发原生 API（不经 `ServiceSlot` 包装，`T: ?Sized` 原生支持 trait object）。
pub trait ServiceCollectionExt {
    /// 注册单例服务（转发 `singleton::<T>`）。`T` 支持 `?Sized`（trait object）。
    fn add_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        factory: impl Fn(&dyn IServiceResolver) -> Arc<T> + Send + Sync + 'static,
    );

    /// 注册 keyed 单例服务（转发 `keyed_singleton::<T>`）。`T` 支持 `?Sized`（trait object）。
    fn add_keyed_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        key: impl Into<String>,
        factory: impl Fn(&dyn IServiceResolver) -> Arc<T> + Send + Sync + 'static,
    );
}

impl ServiceCollectionExt for ServiceCollection {
    fn add_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        factory: impl Fn(&dyn IServiceResolver) -> Arc<T> + Send + Sync + 'static,
    ) {
        let inner = std::mem::replace(self, ServiceCollection::new());
        *self = inner.singleton::<T>(factory);
    }

    fn add_keyed_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        key: impl Into<String>,
        factory: impl Fn(&dyn IServiceResolver) -> Arc<T> + Send + Sync + 'static,
    ) {
        let inner = std::mem::replace(self, ServiceCollection::new());
        *self = inner.keyed_singleton::<T>(key, factory);
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  DixServiceProvider —— rust_dix::ServiceProvider → IServiceProvider
// ──────────────────────────────────────────────────────────────────────────

/// rust-dix `ServiceProvider` 的 RML `IServiceProvider` 桥接。
///
/// `get_service_any` 委托 `get_by_type_id`（O(1) TypeId 直查）。
/// 单例缓存由 rust-dix 内部管理。
pub struct DixServiceProvider {
    inner: Arc<RdiProvider>,
}

impl DixServiceProvider {
    /// 从 `rust_dix::ServiceProvider` 创建桥接。
    pub fn new(inner: Arc<RdiProvider>) -> Self {
        Self { inner }
    }

    /// 构建 ServiceCollection → ServiceProvider → DixServiceProvider。
    ///
    /// 等价 `collection.build()` + `DixServiceProvider::new()`，
    /// 自动收集 `#[inject]` 标记的服务。
    pub fn build(collection: ServiceCollection) -> Arc<dyn IServiceProvider + Send + Sync> {
        let provider = collection
            .build()
            .expect("DI build failed (circular dependency?)");
        Arc::new(Self::new(provider))
    }

    /// 返回内部 `rust_dix::ServiceProvider` 引用（供需要原生 API 的场景使用）。
    pub fn inner(&self) -> &Arc<RdiProvider> {
        &self.inner
    }
}

impl IServiceProvider for DixServiceProvider {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        self.inner.get_by_type_id(type_id)
    }

    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        self.inner.get_keyed_by_type_id(type_id, key)
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.inner.get_by_type_id(type_id).is_some()
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  DixServiceWrapper —— ServiceProviderWrapper → IServiceProvider
// ──────────────────────────────────────────────────────────────────────────

/// rust-dix `ServiceProviderWrapper` 的 RML `IServiceProvider` 桥接。
///
/// child-first 分层解析：优先查找子容器，fallback 到主容器。
/// 用于插件子容器桥接主容器。
pub struct DixServiceWrapper {
    inner: Arc<RdiWrapper>,
}

impl DixServiceWrapper {
    /// 从 `rust_dix::ServiceProviderWrapper` 创建桥接。
    pub fn new(inner: Arc<RdiWrapper>) -> Self {
        Self { inner }
    }

    /// 返回内部 `rust_dix::ServiceProviderWrapper` 引用。
    pub fn inner(&self) -> &Arc<RdiWrapper> {
        &self.inner
    }
}

impl IServiceProvider for DixServiceWrapper {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        self.inner.get_by_type_id(type_id)
    }

    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        self.inner.get_keyed_by_type_id(type_id, key)
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.inner.get_by_type_id(type_id).is_some()
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  自动注册机制 —— #[ctor::ctor] + 全局闭包注册表
// ──────────────────────────────────────────────────────────────────────────
//
// 扩展 crate 经 `#[ctor::ctor]` 调用 `auto_register(closure)` 注册服务,
// `build_runtime_provider` 经 `apply_auto_registrations` 统一应用。
// `Fn`（非 `FnOnce`）+ 非 drain 式读取,支持多次 build。

type RegisterFn = Box<dyn Fn(&mut ServiceCollection) + Send + Sync>;

static AUTO_REGISTRATIONS: OnceLock<Mutex<Vec<RegisterFn>>> = OnceLock::new();

/// 注册自动注册闭包。通常在 `#[ctor::ctor]` 函数中调用。
///
/// 闭包接收 `&mut ServiceCollection`,内部调用 rust-dix 原生
/// `singleton::<dyn T>` / `keyed_singleton::<dyn T>` 注册服务。
/// 闭包为 `Fn`（非 `FnOnce`）,支持多次 build（如测试场景）。
pub fn auto_register(f: impl Fn(&mut ServiceCollection) + Send + Sync + 'static) {
    AUTO_REGISTRATIONS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .push(Box::new(f));
}

/// 应用所有自动注册。`build_runtime_provider` 内部调用。
pub fn apply_auto_registrations(collection: &mut ServiceCollection) {
    if let Some(registry) = AUTO_REGISTRATIONS.get() {
        for f in registry.lock().unwrap().iter() {
            f(collection);
        }
    }
}

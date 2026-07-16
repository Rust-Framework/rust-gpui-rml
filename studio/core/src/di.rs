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
//! ## 手动注册（ServiceSlot 桥接）
//!
//! 对于需要 keyed 注册或复杂工厂的服务，使用 `ServiceSlot<T>` 桥接：
//! ```rust,ignore
//! use studio_core::di::ServiceSlot;
//!
//! s.keyed_singleton::<ServiceSlot<dyn IWorkbenchProvider>>("file", |_| {
//!     Arc::new(ServiceSlot(Arc::new(EditorProvider) as Arc<dyn IWorkbenchProvider>))
//! });
//! ```
//! 查询时经 `cx.get_keyed_trait::<dyn IWorkbenchProvider>("file")` 还原。
//!
//! ## 子主容器桥接（`DixServiceWrapper`）
//!
//! 插件子容器经 `ServiceProviderWrapper` 桥接主容器，child-first 解析：
//! ```rust,ignore
//! let wrapper = ServiceProviderWrapper::new(child_provider, root_provider);
//! let bridge = DixServiceWrapper::new(wrapper);
//! cx.use_provider(bridge);
//! ```

use std::any::{Any, TypeId};
use std::sync::{Arc, Mutex, OnceLock};

use rml_core::context::IServiceProvider;
use rml_core::context::ServiceSlot;
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
//  ServiceSlot 便捷注册扩展
// ──────────────────────────────────────────────────────────────────────────

/// ServiceCollection 扩展 —— ServiceSlot 桥接的便捷注册。
///
/// trait object（`dyn Trait`）经 `ServiceSlot<T>` 包装后注册到 rust-dix，
/// 使 RML 的 `get_trait::<dyn Trait>()` / `get_keyed_trait::<dyn Trait>(key)` 可解析。
pub trait ServiceCollectionExt {
    /// 注册单例 trait object 服务（经 `ServiceSlot<T>` 桥接）。
    fn add_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        factory: impl Fn() -> Arc<T> + Send + Sync + 'static,
    );

    /// 注册 keyed 单例 trait object 服务（经 `ServiceSlot<T>` 桥接）。
    fn add_keyed_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        key: &str,
        factory: impl Fn() -> Arc<T> + Send + Sync + 'static,
    );
}

impl ServiceCollectionExt for ServiceCollection {
    fn add_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        factory: impl Fn() -> Arc<T> + Send + Sync + 'static,
    ) {
        let inner = std::mem::replace(self, ServiceCollection::new());
        *self = inner.singleton::<ServiceSlot<T>>(move |_| {
            let arc_t: Arc<T> = factory();
            Arc::new(ServiceSlot(arc_t))
        });
    }

    fn add_keyed_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        key: &str,
        factory: impl Fn() -> Arc<T> + Send + Sync + 'static,
    ) {
        let inner = std::mem::replace(self, ServiceCollection::new());
        *self = inner.keyed_singleton::<ServiceSlot<T>>(key.to_string(), move |_| {
            let arc_t: Arc<T> = factory();
            Arc::new(ServiceSlot(arc_t))
        });
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
/// 闭包接收 `&mut ServiceCollection`,内部调用 `add_singleton` / `add_keyed_singleton`
/// 注册服务。闭包为 `Fn`（非 `FnOnce`）,支持多次 build（如测试场景）。
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

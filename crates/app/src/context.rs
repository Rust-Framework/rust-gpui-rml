//! 服务容器抽象 —— IServiceProvider（解析）+ IAppContext（GPUI 上下文扩展）
//!
//! 对标 ASP.NET Core DI 抽象：
//! - `IServiceProvider`：纯解析能力（GetService / GetKeyedService / HasService），支持 `?Sized`（trait object）
//! - `RuntimeServiceRegistry`：内置运行时注册表（承载产品层 Sized 服务）
//! - 第三方容器（如 rust-dix）通过实现 `IServiceProvider` 对接，经 `IAppContext::set_provider` 注入
//!
//! 双层查询：正式 provider（rust-dix 等）→ 运行时注册表（RuntimeServiceRegistry）。
//! 产品层 `register_service` 写入运行时注册表，在任何 provider 下都生效。
//!
//! # 分层容器
//!
//! `set_provider` 为覆盖语义（最后注入的 provider 生效），不支持多 provider 串联。
//! 业务方需要多容器分层（如插件子容器桥接主容器）时，应使用 rust-dix 原生
//! `ServiceProviderWrapper`（child-first + root fallback）经 `DixServiceWrapper`
//! 桥接为单一 `IServiceProvider` 后注入，分层逻辑由 rust-dix 原生表达。
//!
//! # Object Safety
//!
//! `IServiceProvider` 通过类型擦除实现 object safety：核心方法 `get_service_any` /
//! `get_keyed_service_any` / `has_service_any` 接收 `TypeId`，返回 `Arc<dyn Any + Send + Sync>`，
//! 可在 `dyn IServiceProvider` 上调用。泛型便利方法 `get_service::<T>()` 等带 `where Self: Sized`
//! 约束，不进入 vtable，由具体类型经默认实现获得。
//!
//! # Trait Object 支持
//!
//! `get_service::<T>()` 的 `T` 支持 `?Sized`，可直接查询 `dyn Trait`：
//! ```rust,ignore
//! let mgr: Arc<dyn IWorkbenchManager> = cx.get_service::<dyn IWorkbenchManager>()?;
//! ```
//! 内部存储格式统一为 `Arc<T>`（`Arc<dyn Trait>` 是 Sized，可擦除为 `Arc<dyn Any + Send + Sync>`），
//! 与 rust-dix `IServiceResolver::get::<T: ?Sized>` 默认实现一致，无需 `ServiceSlot` 包装。

use std::any::{Any, TypeId};
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::{App, BorrowAppContext, Context, Global};

/// 服务提供者抽象 —— 仅解析能力（`&self` 方法）。
///
/// 对标 ASP.NET Core `IServiceProvider` + keyed 服务扩展。
/// 第三方 DI 容器（rust-dix 等）实现此 trait 后，经 `IAppContext::set_provider` 注入。
///
/// 核心方法 `get_service_any` 等是 object-safe 的（类型擦除），泛型便利方法
/// `get_service::<T>()` 等通过 `where Self: Sized` 排除出 vtable，由默认实现委托给 `_any` 方法。
///
/// `T` 支持 `?Sized`，可直接查询 trait object（`dyn Trait`）。
pub trait IServiceProvider {
    /// 类型擦除的服务查询。未注册返回 `None`。
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>>;

    /// 类型擦除的 keyed 服务查询。未注册返回 `None`。
    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>>;

    /// 类型擦除的服务存在性查询。
    fn has_service_any(&self, type_id: TypeId) -> bool;

    /// 查询服务实例。未注册返回 `None`。
    ///
    /// `T` 支持 `?Sized`，可直接查询 trait object（`dyn Trait`）。
    /// 内部存储统一为 `Arc<T>`，downcast 还原后 clone 返回。
    fn get_service<T: ?Sized + 'static + Send + Sync>(&self) -> Option<Arc<T>>
    where
        Self: Sized,
    {
        let any = self.get_service_any(TypeId::of::<T>())?;
        any.downcast::<Arc<T>>().ok().map(|d| Arc::clone(&*d))
    }

    /// 查询 keyed 服务实例。未注册返回 `None`。
    ///
    /// `T` 支持 `?Sized`，可直接查询 trait object（`dyn Trait`）。
    fn get_keyed_service<T: ?Sized + 'static + Send + Sync>(&self, key: &str) -> Option<Arc<T>>
    where
        Self: Sized,
    {
        let any = self.get_keyed_service_any(TypeId::of::<T>(), key)?;
        any.downcast::<Arc<T>>().ok().map(|d| Arc::clone(&*d))
    }

    /// 是否已注册某服务。
    fn has_service<T: ?Sized + 'static + Send + Sync>(&self) -> bool
    where
        Self: Sized,
    {
        self.has_service_any(TypeId::of::<T>())
    }

    /// 查询必需服务。未注册时 panic 并报告类型名。
    fn get_required_service<T: ?Sized + 'static + Send + Sync>(&self) -> Arc<T>
    where
        Self: Sized,
    {
        self.get_service::<T>().unwrap_or_else(|| {
            panic!(
                "required service `{}` not registered",
                std::any::type_name::<T>()
            )
        })
    }

    /// 查询必需 keyed 服务。未注册时 panic。
    fn get_required_keyed_service<T: ?Sized + 'static + Send + Sync>(&self, key: &str) -> Arc<T>
    where
        Self: Sized,
    {
        self.get_keyed_service::<T>(key).unwrap_or_else(|| {
            panic!(
                "required keyed service `{}` (key={}) not registered",
                std::any::type_name::<T>(),
                key
            )
        })
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  自由函数 —— 在 `dyn IServiceProvider` 上查询（绕过 `where Self: Sized` 约束）
// ──────────────────────────────────────────────────────────────────────────
//
// `IServiceProvider::get_service::<T>()` 等泛型方法带 `where Self: Sized` 约束
// （object-safety 要求），无法在 `dyn IServiceProvider` 上调用。当业务类型持有
// `Arc<dyn IServiceProvider>`（如 `ArcShellManager`）时，用这些自由函数查询。
// 类似 `Arc::downcast` / `Any::downcast_ref` 的自由函数模式。

/// 在 `&dyn IServiceProvider` 上查询服务实例。
///
/// 等价 `IServiceProvider::get_service::<T>()`，但可在 `dyn` 上调用。
/// 供持有 `Arc<dyn IServiceProvider>` 的非 GPUI 类型使用。
pub fn resolve_service<T: ?Sized + 'static + Send + Sync>(
    provider: &dyn IServiceProvider,
) -> Option<Arc<T>> {
    let any = provider.get_service_any(TypeId::of::<T>())?;
    any.downcast::<Arc<T>>().ok().map(|d| Arc::clone(&*d))
}

/// 在 `&dyn IServiceProvider` 上查询 keyed 服务实例。
///
/// 等价 `IServiceProvider::get_keyed_service::<T>()`，但可在 `dyn` 上调用。
pub fn resolve_keyed_service<T: ?Sized + 'static + Send + Sync>(
    provider: &dyn IServiceProvider,
    key: &str,
) -> Option<Arc<T>> {
    let any = provider.get_keyed_service_any(TypeId::of::<T>(), key)?;
    any.downcast::<Arc<T>>().ok().map(|d| Arc::clone(&*d))
}

/// 在 `&dyn IServiceProvider` 上查询必需服务。未注册时 panic。
pub fn resolve_required_service<T: ?Sized + 'static + Send + Sync>(
    provider: &dyn IServiceProvider,
) -> Arc<T> {
    resolve_service::<T>(provider).unwrap_or_else(|| {
        panic!(
            "required service `{}` not registered",
            std::any::type_name::<T>()
        )
    })
}

/// 在 `&dyn IServiceProvider` 上查询必需 keyed 服务。未注册时 panic。
pub fn resolve_required_keyed_service<T: ?Sized + 'static + Send + Sync>(
    provider: &dyn IServiceProvider,
    key: &str,
) -> Arc<T> {
    resolve_keyed_service::<T>(provider, key).unwrap_or_else(|| {
        panic!(
            "required keyed service `{}` (key={}) not registered",
            std::any::type_name::<T>(),
            key
        )
    })
}

/// 运行时服务注册表 —— `IServiceProvider` 实现，用作产品层运行时注册表。
///
/// 按 `TypeId` 索引 `Arc<dyn Any + Send + Sync>`，支持 keyed 服务。
/// 接收 `IAppContext::register_service` 调用，在任何正式 provider 下都生效。
#[derive(Default)]
pub struct RuntimeServiceRegistry {
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    keyed: RwLock<HashMap<(TypeId, String), Arc<dyn Any + Send + Sync>>>,
}

impl RuntimeServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册单例服务。重复注册覆盖。
    pub fn set<T: 'static + Send + Sync>(&self, service: Arc<T>) {
        self.services
            .write()
            .unwrap()
            .insert(TypeId::of::<T>(), service);
    }

    /// 注册 keyed 单例服务。重复注册覆盖。
    pub fn set_keyed<T: 'static + Send + Sync>(&self, key: &str, service: Arc<T>) {
        self.keyed
            .write()
            .unwrap()
            .insert((TypeId::of::<T>(), key.to_string()), service);
    }
}

impl IServiceProvider for RuntimeServiceRegistry {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        self.services.read().unwrap().get(&type_id).cloned()
    }

    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        self.keyed
            .read()
            .unwrap()
            .get(&(type_id, key.to_string()))
            .cloned()
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.services.read().unwrap().contains_key(&type_id)
    }
}

/// 服务容器槽位 —— GPUI Global，持有单一 provider + 运行时注册表。
///
/// `provider` 为覆盖语义：`set_provider` 替换原有 provider，不支持多 provider 串联。
/// 多容器分层（如插件子容器桥接主容器）应使用 rust-dix `ServiceProviderWrapper`
/// 经 `DixServiceWrapper` 桥接为单一 `IServiceProvider` 后注入。
///
/// 查询顺序：`provider`（若已注入）→ 运行时注册表（`register_service` 写入）。
struct ServiceProviderSlot {
    /// 正式 provider（覆盖语义）。`Send + Sync` 约束确保 provider 可被
    /// `ArcShellManager` 等 `Send + Sync` 类型持有。
    provider: RwLock<Option<Arc<dyn IServiceProvider + Send + Sync>>>,
    /// 运行时注册表（始终 `RuntimeServiceRegistry`，接收 `register_service` 调用）。
    runtime: RuntimeServiceRegistry,
}

impl ServiceProviderSlot {
    fn new() -> Self {
        Self {
            provider: RwLock::new(None),
            runtime: RuntimeServiceRegistry::new(),
        }
    }

    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        if let Some(p) = self.provider.read().unwrap().as_ref() {
            if let Some(svc) = p.get_service_any(type_id) {
                return Some(svc);
            }
        }
        self.runtime.get_service_any(type_id)
    }

    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        if let Some(p) = self.provider.read().unwrap().as_ref() {
            if let Some(svc) = p.get_keyed_service_any(type_id, key) {
                return Some(svc);
            }
        }
        self.runtime.get_keyed_service_any(type_id, key)
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.provider
            .read()
            .unwrap()
            .as_ref()
            .map(|p| p.has_service_any(type_id))
            .unwrap_or(false)
            || self.runtime.has_service_any(type_id)
    }

    fn set_provider(&self, provider: Arc<dyn IServiceProvider + Send + Sync>) {
        *self.provider.write().unwrap() = Some(provider);
    }
}

impl Global for ServiceProviderSlot {}

/// 确保 `ServiceProviderSlot` Global 已初始化。
pub fn ensure_service_provider(cx: &mut App) {
    if !cx.has_global::<ServiceProviderSlot>() {
        cx.set_global(ServiceProviderSlot::new());
    }
}

/// 应用上下文 —— GPUI App/Context 扩展，持有正式 provider + 运行时注册表。
///
/// 继承 `IServiceProvider` 的解析方法，增加：
/// - `set_provider`：覆盖式注入正式 provider（多容器分层经 `ServiceProviderWrapper` 原生表达）
/// - `register_service`：运行时注册（写入运行时注册表，任何 provider 下都生效）
pub trait IAppContext: IServiceProvider {
    /// 注入正式 provider（覆盖语义）。
    /// `Send + Sync` 约束确保 provider 可被 `Send + Sync` 类型（如 `ArcShellManager`）持有。
    fn set_provider(&mut self, provider: Arc<dyn IServiceProvider + Send + Sync>);

    /// 运行时注册服务（写入运行时注册表）。
    fn register_service<T: 'static + Send + Sync>(&mut self, service: Arc<T>);
}

impl IServiceProvider for App {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        self.try_global::<ServiceProviderSlot>()?.get_service_any(type_id)
    }

    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        self.try_global::<ServiceProviderSlot>()?.get_keyed_service_any(type_id, key)
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.try_global::<ServiceProviderSlot>()
            .map(|slot| slot.has_service_any(type_id))
            .unwrap_or(false)
    }
}

impl IAppContext for App {
    fn set_provider(&mut self, provider: Arc<dyn IServiceProvider + Send + Sync>) {
        ensure_service_provider(self);
        self.update_global::<ServiceProviderSlot, _>(|slot, _| {
            slot.set_provider(provider);
        });
    }

    fn register_service<T: 'static + Send + Sync>(&mut self, service: Arc<T>) {
        ensure_service_provider(self);
        self.update_global::<ServiceProviderSlot, _>(|slot, _| {
            slot.runtime.set(service);
        });
    }
}

impl<T> IServiceProvider for Context<'_, T> {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        IServiceProvider::get_service_any(Borrow::<App>::borrow(self), type_id)
    }

    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        IServiceProvider::get_keyed_service_any(Borrow::<App>::borrow(self), type_id, key)
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        IServiceProvider::has_service_any(Borrow::<App>::borrow(self), type_id)
    }
}

impl<T> IAppContext for Context<'_, T> {
    fn set_provider(&mut self, provider: Arc<dyn IServiceProvider + Send + Sync>) {
        IAppContext::set_provider(BorrowMut::<App>::borrow_mut(self), provider);
    }

    fn register_service<U: 'static + Send + Sync>(&mut self, service: Arc<U>) {
        IAppContext::register_service(BorrowMut::<App>::borrow_mut(self), service);
    }
}

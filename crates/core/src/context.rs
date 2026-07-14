//! 服务容器抽象 —— IServiceProvider（解析）+ IAppContext（GPUI 上下文扩展）
//!
//! 对标 ASP.NET Core DI 抽象：
//! - `IServiceProvider`：纯解析能力（GetService / GetKeyedService / HasService）
//! - `DefaultServiceProvider`：内置默认实现（类似 MEDI）
//! - 第三方容器（如 rust-dix）通过实现 `IServiceProvider` 对接，经 `IAppContext::use_provider` 注入
//!
//! 双层查询：正式后端（rust-dix 等）→ 运行时注册表（DefaultServiceProvider）。
//! 框架内部 `set_service` 写入运行时注册表，在任何后端下都生效。
//!
//! # Object Safety
//!
//! `IServiceProvider` 通过类型擦除实现 object safety：核心方法 `get_service_any` /
//! `get_keyed_service_any` / `has_service_any` 接收 `TypeId`，返回 `Arc<dyn Any + Send + Sync>`，
//! 可在 `dyn IServiceProvider` 上调用。泛型便利方法 `get_service::<T>()` 等带 `where Self: Sized`
//! 约束，不进入 vtable，由具体类型经默认实现获得。

use std::any::{Any, TypeId};
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::{App, BorrowAppContext, Context, Global};

/// 服务提供者抽象 —— 仅解析能力（`&self` 方法）。
///
/// 对标 ASP.NET Core `IServiceProvider` + keyed 服务扩展。
/// 第三方 DI 容器（rust-dix 等）实现此 trait 后，经 `IAppContext::use_provider` 注入。
///
/// 核心方法 `get_service_any` 等是 object-safe 的（类型擦除），泛型便利方法
/// `get_service::<T>()` 等通过 `where Self: Sized` 排除出 vtable，由默认实现委托给 `_any` 方法。
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
    fn get_service<T: 'static + Send + Sync>(&self) -> Option<Arc<T>>
    where
        Self: Sized,
    {
        self.get_service_any(TypeId::of::<T>())
            .and_then(|any| any.downcast::<T>().ok())
    }

    /// 查询 keyed 服务实例。未注册返回 `None`。
    fn get_keyed_service<T: 'static + Send + Sync>(&self, key: &str) -> Option<Arc<T>>
    where
        Self: Sized,
    {
        self.get_keyed_service_any(TypeId::of::<T>(), key)
            .and_then(|any| any.downcast::<T>().ok())
    }

    /// 是否已注册某服务。
    fn has_service<T: 'static + Send + Sync>(&self) -> bool
    where
        Self: Sized,
    {
        self.has_service_any(TypeId::of::<T>())
    }

    /// 查询必需服务。未注册时 panic 并报告类型名。
    fn get_required_service<T: 'static + Send + Sync>(&self) -> Arc<T>
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
    fn get_required_keyed_service<T: 'static + Send + Sync>(&self, key: &str) -> Arc<T>
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

/// Trait object 服务槽位 —— 将 `Arc<dyn Trait>` 包装为 Sized 类型，
/// 使其可通过 `IServiceProvider` 的类型擦除层注册/查询。
///
/// `dyn Trait` 是 `!Sized`，无法直接 `downcast`。`ServiceSlot<dyn Trait>`
/// 是 Sized 结构体（包裹 `Arc<dyn Trait>`），可存入 `Arc<dyn Any + Send + Sync>`
/// 并经 `downcast::<ServiceSlot<dyn Trait>>()` 还原。
///
/// 查询模式：
/// - 具体类型：`cx.get_service::<MyStruct>()` — 直接查询
/// - Trait object：`cx.get_trait::<dyn ITrait>()` — 经 ServiceSlot 桥接（见 `ServiceProviderExt`）
pub struct ServiceSlot<T: ?Sized + 'static + Send + Sync>(pub Arc<T>);

/// `IServiceProvider` 便捷扩展 —— trait object 查询经 `ServiceSlot` 桥接。
///
/// blanket impl 对所有 `IServiceProvider` 实现（含 `dyn IServiceProvider`），
/// 业务代码 `use` 此 trait 后即可调用 `get_trait` / `get_keyed_trait`。
pub trait ServiceProviderExt: IServiceProvider {
    /// 查询 trait object 服务（经 `ServiceSlot` 桥接）。
    fn get_trait<T: ?Sized + 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        self.get_service_any(TypeId::of::<ServiceSlot<T>>())
            .and_then(|any| any.downcast::<ServiceSlot<T>>().ok())
            .map(|slot| slot.0.clone())
    }

    /// 查询 keyed trait object 服务（经 `ServiceSlot` 桥接）。
    fn get_keyed_trait<T: ?Sized + 'static + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.get_keyed_service_any(TypeId::of::<ServiceSlot<T>>(), key)
            .and_then(|any| any.downcast::<ServiceSlot<T>>().ok())
            .map(|slot| slot.0.clone())
    }

    /// 查询必需 trait object 服务。未注册时 panic。
    fn get_required_trait<T: ?Sized + 'static + Send + Sync>(&self) -> Arc<T> {
        self.get_trait::<T>().unwrap_or_else(|| {
            panic!(
                "required trait service `{}` not registered",
                std::any::type_name::<T>()
            )
        })
    }
}

impl<S: IServiceProvider + ?Sized> ServiceProviderExt for S {}

/// 默认服务提供者 —— core 内置的简陋实现（原 `ServiceCollection` 重命名）。
///
/// 按 `TypeId` 索引 `Arc<dyn Any + Send + Sync>`，支持 keyed 服务。
/// 作为 `IServiceProvider` 的默认后端，也用作运行时注册表。
#[derive(Default)]
pub struct DefaultServiceProvider {
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    keyed: RwLock<HashMap<(TypeId, String), Arc<dyn Any + Send + Sync>>>,
}

impl DefaultServiceProvider {
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

impl IServiceProvider for DefaultServiceProvider {
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

/// 服务容器槽位 —— GPUI Global，持有 provider 链 + 运行时注册表。
///
/// provider 链支持多阶段注入：`configure` 阶段（启动前）与 `on_loaded` 阶段
/// （窗口加载后）的 provider 共存，查询时依次尝试。解决 `ArcShellManager` 等
/// 循环依赖服务的二阶段注入问题。
///
/// 查询顺序：provider 链（按注入顺序）→ 运行时注册表（`set_service` 写入）。
struct ServiceProviderSlot {
    /// provider 链：`configure` + `on_loaded` 阶段注入的 provider 依次查询。
    /// `Send + Sync` 约束确保 provider 可被 `ArcShellManager` 等 `Send + Sync` 类型持有。
    providers: RwLock<Vec<Arc<dyn IServiceProvider + Send + Sync>>>,
    /// 运行时注册表（始终 `DefaultServiceProvider`，接收 `set_service` 调用）。
    runtime: DefaultServiceProvider,
}

impl ServiceProviderSlot {
    fn new() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
            runtime: DefaultServiceProvider::new(),
        }
    }

    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        for p in self.providers.read().unwrap().iter() {
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
        for p in self.providers.read().unwrap().iter() {
            if let Some(svc) = p.get_keyed_service_any(type_id, key) {
                return Some(svc);
            }
        }
        self.runtime.get_keyed_service_any(type_id, key)
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.providers
            .read()
            .unwrap()
            .iter()
            .any(|p| p.has_service_any(type_id))
            || self.runtime.has_service_any(type_id)
    }

    fn use_provider(&self, provider: Arc<dyn IServiceProvider + Send + Sync>) {
        self.providers.write().unwrap().push(provider);
    }
}

impl Global for ServiceProviderSlot {}

/// 确保 `ServiceProviderSlot` Global 已初始化。
pub fn ensure_service_provider(cx: &mut App) {
    if !cx.has_global::<ServiceProviderSlot>() {
        cx.set_global(ServiceProviderSlot::new());
    }
}

/// 应用上下文 —— GPUI App/Context 扩展，持有 provider 链 + 运行时注册表。
///
/// 继承 `IServiceProvider` 的解析方法，增加：
/// - `use_provider`：追加 provider 到链（支持多阶段注入）
/// - `set_service`：运行时注册（写入运行时注册表，任何 provider 链下都生效）
pub trait IAppContext: IServiceProvider {
    /// 追加 provider 到 provider 链。支持 `configure` + `on_loaded` 多阶段注入。
    /// `Send + Sync` 约束确保 provider 可被 `Send + Sync` 类型（如 `ArcShellManager`）持有。
    fn use_provider(&mut self, provider: Arc<dyn IServiceProvider + Send + Sync>);

    /// 运行时注册服务（写入运行时注册表）。
    fn set_service<T: 'static + Send + Sync>(&mut self, service: Arc<T>);
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
    fn use_provider(&mut self, provider: Arc<dyn IServiceProvider + Send + Sync>) {
        ensure_service_provider(self);
        self.update_global::<ServiceProviderSlot, _>(|slot, _| {
            slot.use_provider(provider);
        });
    }

    fn set_service<T: 'static + Send + Sync>(&mut self, service: Arc<T>) {
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
    fn use_provider(&mut self, provider: Arc<dyn IServiceProvider + Send + Sync>) {
        IAppContext::use_provider(BorrowMut::<App>::borrow_mut(self), provider);
    }

    fn set_service<U: 'static + Send + Sync>(&mut self, service: Arc<U>) {
        IAppContext::set_service(BorrowMut::<App>::borrow_mut(self), service);
    }
}

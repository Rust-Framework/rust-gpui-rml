//! IAppContext —— 贯穿整个 RML 应用的统一上下文接口（IServiceProvider 风格）
//!
//! 借鉴 C# `System.IServiceProvider`：所有全局服务（注册表、管理器、业务单例）
//! 通过 `get_service::<T>()` 动态查询。框架提供 trait + `ServiceCollection` 存储，
//! 为 `App` 和 `Context<'_, T>` 同时实现，业务代码可在任意上下文统一访问。
//!
//! 三方法对应关系：
//! - `get_service::<T>()` 类比 `GetService<T>()` —— 可选查询
//! - `get_required_service::<T>()` 类比 `GetRequiredService<T>()` —— 必需服务（panic）
//! - `set_service::<T>(instance)` 类比 `TryAddSingleton<T>(instance)` —— 注册单例

use std::any::{Any, TypeId};
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::{App, BorrowAppContext, Context, Global};

/// 服务集合——`IAppContext` 的存储后端。
///
/// 按 `TypeId` 索引 `Arc<dyn Any + Send + Sync>`，内部 `RwLock` 可变性。
/// 作为 GPUI `Global` 存储，借用 `App`，支持 `observe_global` 触发刷新。
#[derive(Default)]
pub struct ServiceCollection {
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl ServiceCollection {
    /// 查询服务实例。返回 `Option<Arc<T>>`——未注册返回 `None`。
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        self.services
            .read()
            .unwrap()
            .get(&TypeId::of::<T>())
            .and_then(|any| any.clone().downcast::<T>().ok())
    }

    /// 注册单例服务。重复注册覆盖旧实例。
    pub fn set<T: 'static + Send + Sync>(&self, service: Arc<T>) {
        self.services
            .write()
            .unwrap()
            .insert(TypeId::of::<T>(), service);
    }

    /// 是否已注册某服务。
    pub fn contains<T: 'static + Send + Sync>(&self) -> bool {
        self.services
            .read()
            .unwrap()
            .contains_key(&TypeId::of::<T>())
    }
}

impl Global for ServiceCollection {}

/// IAppContext——贯穿整个 RML 应用的统一上下文接口。
///
/// 为 `App` 和 `Context<'_, T>` 同时实现，业务代码 `cx.get_service::<T>()`
/// 在任意上下文（启动回调、ViewModel、命令处理器）统一可用。
pub trait IAppContext {
    /// 查询服务实例。未注册返回 `None`。
    fn get_service<T: 'static + Send + Sync>(&self) -> Option<Arc<T>>;

    /// 查询必需服务。未注册时 panic 并报告类型名。
    fn get_required_service<T: 'static + Send + Sync>(&self) -> Arc<T> {
        self.get_service::<T>().unwrap_or_else(|| {
            panic!(
                "required service `{}` not registered in IAppContext",
                std::any::type_name::<T>()
            )
        })
    }

    /// 注册单例服务。
    fn set_service<T: 'static + Send + Sync>(&mut self, service: Arc<T>);

    /// 是否已注册某服务。
    fn has_service<T: 'static + Send + Sync>(&self) -> bool;
}

/// 确保 `ServiceCollection` Global 已初始化。
pub fn ensure_service_collection(cx: &mut App) {
    if !cx.has_global::<ServiceCollection>() {
        cx.set_global(ServiceCollection::default());
    }
}

impl IAppContext for App {
    fn get_service<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        self.try_global::<ServiceCollection>()
            .and_then(|sc| sc.get::<T>())
    }

    fn set_service<T: 'static + Send + Sync>(&mut self, service: Arc<T>) {
        ensure_service_collection(self);
        self.update_global::<ServiceCollection, _>(|sc, _| sc.set(service));
    }

    fn has_service<T: 'static + Send + Sync>(&self) -> bool {
        self.try_global::<ServiceCollection>()
            .map(|sc| sc.contains::<T>())
            .unwrap_or(false)
    }
}

impl<T> IAppContext for Context<'_, T> {
    fn get_service<U: 'static + Send + Sync>(&self) -> Option<Arc<U>> {
        IAppContext::get_service::<U>(Borrow::<App>::borrow(self))
    }

    fn set_service<U: 'static + Send + Sync>(&mut self, service: Arc<U>) {
        IAppContext::set_service::<U>(BorrowMut::<App>::borrow_mut(self), service);
    }

    fn has_service<U: 'static + Send + Sync>(&self) -> bool {
        IAppContext::has_service::<U>(Borrow::<App>::borrow(self))
    }
}

//! ServiceProvider —— 解析容器（impl IServiceProvider，自维护 cache）
//!
//! 对标 ASP.NET Core `ServiceProvider`。持有 factory map + singleton cache，
//! `get_service_any` 先查缓存（单例语义），未命中则调用 factory（传入 `self`
//! as `&dyn IServiceProvider`，支持 factory 内依赖注入），缓存后返回。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rml_core::context::IServiceProvider;

use crate::collection::FactoryFn;

/// 服务解析容器 —— 持有 factory map + singleton cache。
///
/// `get_service_any` 先查缓存（单例语义），未命中则调用 factory。
/// factory 接收 `self as &dyn IServiceProvider`，支持 factory 内递归解析
/// （经 `ServiceProviderExt::get_trait` 查询其他服务）。
pub struct ServiceProvider {
    pub(crate) factories: HashMap<TypeId, FactoryFn>,
    pub(crate) keyed_factories: HashMap<(TypeId, String), FactoryFn>,
    pub(crate) cache: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    pub(crate) keyed_cache: RwLock<HashMap<(TypeId, String), Arc<dyn Any + Send + Sync>>>,
}

impl IServiceProvider for ServiceProvider {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        if let Some(cached) = self.cache.read().unwrap().get(&type_id) {
            return Some(cached.clone());
        }
        let factory = self.factories.get(&type_id)?;
        let instance = factory(self as &dyn IServiceProvider);
        Some(
            self.cache
                .write()
                .unwrap()
                .entry(type_id)
                .or_insert(instance)
                .clone(),
        )
    }

    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        let cache_key = (type_id, key.to_string());
        if let Some(cached) = self.keyed_cache.read().unwrap().get(&cache_key) {
            return Some(cached.clone());
        }
        let factory = self.keyed_factories.get(&cache_key)?;
        let instance = factory(self as &dyn IServiceProvider);
        Some(
            self.keyed_cache
                .write()
                .unwrap()
                .entry(cache_key)
                .or_insert(instance)
                .clone(),
        )
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.factories.contains_key(&type_id)
    }
}

//! 视觉贡献 Entity 生命周期管理（基于 IAppContext::IServiceProvider）
//!
//! `IVisualContribution::render` 通过 `get_or_create_entity::<T>(cx)` 复用 Entity，
//! 避免每次渲染创建新实例导致状态丢失。强引用 `Entity<T>` 在应用生命周期内不被释放，
//! `on_loaded` 只触发一次。
//!
//! **语义澄清**：本模块管理的是视觉贡献的渲染 Entity 生命周期（防止状态丢失），
//! **不是**贡献注册缓存。贡献注册数据由 `IContributionHost` 直接管理（框架不存储）。
//! 两者职责正交：Host 管"有哪些贡献"，本模块管"视觉贡献的 Entity 不被重建"。
//!
//! 内部存储统一到 `IServiceProvider`（通过 `IAppContext::set_service` 注册），
//! 与 i18n/theme 范式对齐。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::{App, AppContext};
use rml_core::context::{IAppContext, IServiceProvider};

type CacheMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

/// 视觉贡献 Entity 生命周期管理器（存入 `IServiceProvider` 作为单例服务）。
///
/// 管理 `IVisualContribution::render` 产生的 Entity，确保同一视觉贡献类型
/// 在多次渲染间复用同一 Entity，避免状态丢失。不存储贡献注册数据。
pub struct VisualEntityCache {
    inner: RwLock<CacheMap>,
}

impl VisualEntityCache {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for VisualEntityCache {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_cache(cx: &mut App) -> Arc<VisualEntityCache> {
    if let Some(c) = cx.get_service::<VisualEntityCache>() {
        return c;
    }
    let c = Arc::new(VisualEntityCache::new());
    cx.set_service(c.clone());
    c
}

/// 获取或创建视觉贡献的缓存 Entity。
///
/// 首次调用创建 Entity 并缓存强引用 `Entity<T>`；后续调用 clone 复用。
/// 强引用保证 Entity 不被释放，`on_loaded` 只触发一次（避免状态丢失）。
/// Entity 的 `on_loaded` 由自动生成的 `Render::render` 经 `__rml_state.loaded` 标志触发。
pub fn get_or_create_entity<T>(cx: &mut App) -> gpui::Entity<T>
where
    T: 'static + Send + Sync + Default,
{
    let type_id = TypeId::of::<T>();
    let cache = ensure_cache(cx);
    {
        let guard = cache.inner.read().unwrap();
        if let Some(entry) = guard.get(&type_id) {
            if let Some(entity) = entry.downcast_ref::<gpui::Entity<T>>() {
                return entity.clone();
            }
        }
    }
    let entity = cx.new(|_| T::default());
    cache
        .inner
        .write()
        .unwrap()
        .insert(type_id, Box::new(entity.clone()));
    entity
}

/// 获取已缓存的视觉贡献 Entity（不存在则创建）。用于 observe。
pub fn visual_entity<T>(cx: &mut App) -> gpui::Entity<T>
where
    T: 'static + Send + Sync + Default,
{
    get_or_create_entity::<T>(cx)
}

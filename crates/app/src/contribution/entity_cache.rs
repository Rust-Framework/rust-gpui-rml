//! 视觉贡献实体缓存（基于 IAppContext::ServiceCollection）
//!
//! `IVisualContribution::render` 通过 `get_or_create_entity::<T>(cx)` 复用 Entity，
//! 避免每次渲染创建新实例导致状态丢失。缓存以强引用 `Entity<T>` 存储，
//! 保证 Entity 在应用生命周期内不被释放，`on_loaded` 只触发一次。
//!
//! 内部存储统一到 `ServiceCollection`（通过 `IAppContext::set_service` 注册），
//! 不再使用独立 `OnceLock` 静态全局，与 i18n/theme/contribution 范式对齐。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::{App, AppContext};
use rml_core::context::IAppContext;

type CacheMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

/// 视觉贡献 Entity 缓存（存入 `ServiceCollection` 作为单例服务）。
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

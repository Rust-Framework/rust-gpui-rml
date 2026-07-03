//! 视觉贡献实体缓存
//!
//! `IVisualContribution::render` 通过 `get_or_create_entity::<T>(cx)` 复用 Entity，
//! 避免每次渲染创建新实例导致状态丢失。缓存以 `WeakEntity<T>` 存储，
//! Entity 释放后自动失效，下次调用时重建。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use gpui::{App, AppContext};

type CacheMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

static CACHE: OnceLock<RwLock<CacheMap>> = OnceLock::new();

fn cache() -> &'static RwLock<CacheMap> {
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 获取或创建视觉贡献的缓存 Entity。
///
/// 首次调用创建 Entity 并缓存 `WeakEntity<T>`；后续调用 upgrade 复用。
/// Entity 的 `on_loaded` 由自动生成的 `Render::render` 经 `__rml_loaded` 标志触发。
pub fn get_or_create_entity<T>(cx: &mut App) -> gpui::Entity<T>
where
    T: 'static + Send + Sync + Default,
{
    let type_id = TypeId::of::<T>();
    {
        let cache = cache().read().unwrap();
        if let Some(entry) = cache.get(&type_id) {
            if let Some(weak) = entry.downcast_ref::<gpui::WeakEntity<T>>() {
                if let Some(entity) = weak.upgrade() {
                    return entity;
                }
            }
        }
    }
    let entity = cx.new(|_| T::default());
    let weak = entity.downgrade();
    cache().write().unwrap().insert(type_id, Box::new(weak));
    entity
}

/// 获取已缓存的视觉贡献 Entity（不存在则创建）。用于 observe。
pub fn visual_entity<T>(cx: &mut App) -> gpui::Entity<T>
where
    T: 'static + Send + Sync + Default,
{
    get_or_create_entity::<T>(cx)
}

//! 视觉贡献 Entity 生命周期管理（基于 GPUI Global）
//!
//! `IVisualContribution::render` 通过 `get_or_create_entity::<T>(cx)` 复用 Entity，
//! 避免每次渲染创建新实例导致状态丢失。强引用 `Entity<T>` 在应用生命周期内不被释放，
//! `on_loaded` 只触发一次。
//!
//! **语义澄清**：本模块管理的是视觉贡献的渲染 Entity 生命周期（防止状态丢失），
//! **不是**贡献注册缓存。贡献注册数据由 `IContributionHost` 直接管理（框架不存储）。
//! 两者职责正交：Host 管"有哪些贡献"，本模块管"视觉贡献的 Entity 不被重建"。
//!
//! 存储形态：`VisualEntityCache` 作为 GPUI Global 存储（`ensure_cache` 经
//! `try_global`/`set_global` 懒初始化），框架内部服务不经 `IServiceProvider`，
//! 与 i18n/theme/ContributionRegistry 范式对齐。
//! newtype `VisualEntityCache(Arc<Inner>)` 使其 `Clone` 并可独立实现 `Global`（绕开 orphan rule）。
//!
//! # 双层缓存策略
//!
//! - **TypeId 键（单例）**：普通组件经 `get_or_create_entity::<T>` 复用，全类型共享一个 Entity。
//! - **URI 键（多实例）**：IWorkbench 经 `get_or_create_entity_by_uri::<T>(uri)` 复用，
//!   每个 URI 独立持久化 Entity（切 Tab 不丢失状态）。
//!
//! 活跃 Entity 追踪（`get_active_entity`）供子组件查找当前渲染的 host：
//! `IVisual::render` 调用 `get_or_create_entity_by_uri` 时自动设为活跃，
//! 子组件 `before_render` 经 `get_active_entity::<HostType>` 读取。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::{App, AppContext, Global};

type CacheMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;
type UriCacheMap = HashMap<(TypeId, String), Box<dyn Any + Send + Sync>>;
type ActiveMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

/// 视觉贡献 Entity 生命周期管理器（作为 GPUI Global 存储懒初始化）。
///
/// newtype 包装 `Arc<Inner>`：`Clone` 为浅拷贝（引用计数递增），并使本类型可独立实现 `Global`
/// （绕开为 `Arc<T>` 实现 trait 的 orphan rule 限制）。
///
/// 管理三层缓存：
/// - `inner`：TypeId 键单例缓存（普通组件）
/// - `uri_inner`：URI 键多实例缓存（IWorkbench，每 URI 独立 Entity）
/// - `active`：活跃 Entity 追踪（最近一次 `get_or_create_entity_by_uri` 的结果，供子组件查找 host）
#[derive(Clone)]
pub struct VisualEntityCache(Arc<VisualEntityCacheInner>);

struct VisualEntityCacheInner {
    inner: RwLock<CacheMap>,
    uri_inner: RwLock<UriCacheMap>,
    active: RwLock<ActiveMap>,
}

/// 框架内部服务经 GPUI Global 存储（不经过 IServiceProvider）。
/// 由 `ensure_cache` 懒初始化（首次访问时 `set_global`）。
impl Global for VisualEntityCache {}

impl VisualEntityCache {
    pub fn new() -> Self {
        Self(Arc::new(VisualEntityCacheInner {
            inner: RwLock::new(HashMap::new()),
            uri_inner: RwLock::new(HashMap::new()),
            active: RwLock::new(HashMap::new()),
        }))
    }
}

impl Default for VisualEntityCache {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_cache(cx: &mut App) -> VisualEntityCache {
    if let Some(c) = cx.try_global::<VisualEntityCache>() {
        return c.clone();
    }
    let c = VisualEntityCache::new();
    cx.set_global(c.clone());
    c
}

/// 获取或创建视觉贡献的缓存 Entity（TypeId 键单例）。
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
        let guard = cache.0.inner.read().unwrap();
        if let Some(entry) = guard.get(&type_id) {
            if let Some(entity) = entry.downcast_ref::<gpui::Entity<T>>() {
                return entity.clone();
            }
        }
    }
    let entity = cx.new(|_| T::default());
    cache
        .0
        .inner
        .write()
        .unwrap()
        .insert(type_id, Box::new(entity.clone()));
    entity
}

/// 获取或创建 IWorkbench 的 URI 键缓存 Entity。
///
/// 每个 URI 独立持久化 Entity（切 Tab 不丢失状态）。首次调用创建 `T::default()`，
/// 后续同 URI 调用 clone 复用。同时将该 Entity 设为"活跃"，供子组件经
/// [`get_active_entity`] 查找当前渲染的 host。
///
/// 外部实例（Provider 创建）的数据同步由 `ILifecycle::sync_from_external` 负责，
/// 在 `Render::render` 之前调用。
pub fn get_or_create_entity_by_uri<T>(uri: &str, cx: &mut App) -> gpui::Entity<T>
where
    T: 'static + Send + Sync + Default,
{
    let key = (TypeId::of::<T>(), uri.to_string());
    let cache = ensure_cache(cx);
    {
        let guard = cache.0.uri_inner.read().unwrap();
        if let Some(entry) = guard.get(&key) {
            if let Some(entity) = entry.downcast_ref::<gpui::Entity<T>>() {
                set_active::<T>(entity.clone(), &cache);
                return entity.clone();
            }
        }
    }
    let entity = cx.new(|_| T::default());
    cache
        .0
        .uri_inner
        .write()
        .unwrap()
        .insert(key, Box::new(entity.clone()));
    set_active::<T>(entity.clone(), &cache);
    entity
}

/// 获取当前活跃的 T 类型 Entity（子组件查找 host 用）。
///
/// 返回最近一次 [`get_or_create_entity_by_uri`] 设置的活跃 Entity。
/// 在 `IVisual::render` → `entity.update` → `before_render` 链路中，
/// `set_active` 在 `entity.update` 之前执行，`get_active_entity` 在
/// `before_render` 内部调用，时序安全。
///
/// 返回 `None` 表示该类型尚无活跃 Entity（首次渲染前或非 IWorkbench 类型）。
pub fn get_active_entity<T>(cx: &mut App) -> Option<gpui::Entity<T>>
where
    T: 'static + Send + Sync,
{
    let cache = ensure_cache(cx);
    let guard = cache.0.active.read().unwrap();
    guard
        .get(&TypeId::of::<T>())
        .and_then(|e| e.downcast_ref::<gpui::Entity<T>>())
        .cloned()
}

/// 关闭工作台时清理 URI 键缓存（防止内存泄漏）。
///
/// 在 Tab 关闭回调中调用（有 `&mut App` 的时机），移除对应 URI 的 Entity 缓存。
/// 不清理 `active` —— 若关闭的是当前活跃 Tab，下次渲染会自然替换。
pub fn evict_entity_by_uri<T>(uri: &str, cx: &mut App)
where
    T: 'static,
{
    let key = (TypeId::of::<T>(), uri.to_string());
    let cache = ensure_cache(cx);
    cache.0.uri_inner.write().unwrap().remove(&key);
}

fn set_active<T: 'static>(entity: gpui::Entity<T>, cache: &VisualEntityCache) {
    cache
        .0
        .active
        .write()
        .unwrap()
        .insert(TypeId::of::<T>(), Box::new(entity));
}

/// 获取已缓存的视觉贡献 Entity（不存在则创建）。用于 observe。
pub fn visual_entity<T>(cx: &mut App) -> gpui::Entity<T>
where
    T: 'static + Send + Sync + Default,
{
    get_or_create_entity::<T>(cx)
}

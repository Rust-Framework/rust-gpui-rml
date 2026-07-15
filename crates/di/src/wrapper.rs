//! ServiceProviderWrapper —— 子主容器桥接（基于 rust-dix）
//!
//! 包装 `rust_dix::ServiceProviderWrapper`，提供 child-first 分层解析：
//! 子容器先查，未命中回退根容器。用于插件子容器桥接主容器场景。
//!
//! 对外 impl RML `IServiceProvider`，经 `get_by_type_id` 完成类型擦除查询。

use std::any::{Any, TypeId};
use std::sync::Arc;

use rml_core::context::IServiceProvider;
use rust_dix::entry::IServiceResolver;
use rust_dix::ServiceProvider as RdiProvider;
use rust_dix::ServiceProviderWrapper as RdiWrapper;

/// 子主容器桥接 —— child-first 分层解析。
///
/// 构造时接收子容器与根容器（均为 `rust_dix::ServiceProvider`），
/// `get_service_any` 委托 rust-dix wrapper 的 `get_by_type_id`，
/// 自动 child-first → root fallback。
pub struct ServiceProviderWrapper {
    inner: Arc<RdiWrapper>,
}

impl ServiceProviderWrapper {
    /// 构造桥接容器。`child` 优先解析，`root` 作为回退。
    pub fn new(child: Arc<RdiProvider>, root: Arc<RdiProvider>) -> Arc<Self> {
        Arc::new(Self {
            inner: RdiWrapper::new(child, root),
        })
    }
}

impl IServiceProvider for ServiceProviderWrapper {
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

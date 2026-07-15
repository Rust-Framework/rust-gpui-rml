//! ServiceProvider —— 解析容器（基于 rust-dix，impl IServiceProvider）
//!
//! 对标 ASP.NET Core `ServiceProvider`。内部委托 `rust_dix::ServiceProvider`，
//! 对外 impl RML `IServiceProvider`，经 `get_by_type_id` 完成类型擦除查询。
//!
//! 单例缓存由 rust-dix 内部管理，无需自维护 cache。

use std::any::{Any, TypeId};
use std::sync::Arc;

use rml_core::context::IServiceProvider;
use rust_dix::entry::IServiceResolver;
use rust_dix::ServiceProvider as RdiProvider;

/// 服务解析容器 —— 包装 `rust_dix::ServiceProvider`，impl RML `IServiceProvider`。
///
/// `get_service_any` 委托 rust-dix 的 `get_by_type_id`，单例缓存由 rust-dix 内部管理。
pub struct ServiceProvider {
    pub(crate) inner: Arc<RdiProvider>,
}

impl IServiceProvider for ServiceProvider {
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

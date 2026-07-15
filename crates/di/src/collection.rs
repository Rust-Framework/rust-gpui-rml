//! ServiceCollection —— 注册容器（基于 rust-dix）
//!
//! 对标 ASP.NET Core `IServiceCollection`。内部委托 `rust_dix::ServiceCollection`，
//! 对外保持 RML 风格 API（`add_singleton` / `add_keyed_singleton`）。
//!
//! 所有服务经 `ServiceSlot<T>` 桥接存储，使 trait object（`dyn Trait`）可通过
//! `IServiceProvider` 的类型擦除层注册/查询。查询时经 `ServiceProviderExt::get_trait`
//! 还原为 `Arc<T>`。
//!
//! `new()` 时自动调用 `rust_dix::ServiceCollection::from_injected()` 收集所有
//! `#[inject]` 标记的自动注册服务，与手动 `add_singleton` 注册合并。

use std::any::TypeId;
use std::sync::Arc;

use rml_core::context::{IServiceProvider, ServiceSlot};
use rust_dix::entry::IServiceResolver;
use rust_dix::ServiceCollection as RdiCollection;

/// 服务注册容器 —— 基于 rust-dix，按 `TypeId` 索引 factory。
///
/// 所有服务经 `ServiceSlot<T>` 桥接：`add_singleton::<dyn ITrait>(factory)` 注册时，
/// factory 返回的 `Arc<dyn ITrait>` 被包装为 `ServiceSlot<dyn ITrait>`（Sized），
/// 存入 `factories[TypeId::of::<ServiceSlot<dyn ITrait>>()]`。
/// 查询时经 `get_trait::<dyn ITrait>()` 还原。
///
/// `new()` 自动收集 `#[inject]` 标记的服务（经 `from_injected()`）。
pub struct ServiceCollection {
    inner: RdiCollection,
}

impl ServiceCollection {
    pub fn new() -> Self {
        Self {
            inner: RdiCollection::from_injected(),
        }
    }

    /// 注册单例服务。`factory` 接收 `&dyn IServiceProvider` 支持工厂内依赖注入。
    ///
    /// 服务经 `ServiceSlot<T>` 桥接存储，查询时经 `get_trait::<T>()` 还原。
    pub fn add_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        factory: impl Fn(&dyn IServiceProvider) -> Arc<T> + Send + Sync + 'static,
    ) {
        let inner = std::mem::replace(&mut self.inner, RdiCollection::new());
        self.inner = inner.singleton::<ServiceSlot<T>>(move |resolver: &dyn IServiceResolver| {
            let adapter = ResolverAdapter { resolver };
            let arc_t: Arc<T> = factory(&adapter);
            Arc::new(ServiceSlot(arc_t))
        });
    }

    /// 注册 keyed 单例服务。`key` 区分同一 trait 的多个实现。
    pub fn add_keyed_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        key: &str,
        factory: impl Fn(&dyn IServiceProvider) -> Arc<T> + Send + Sync + 'static,
    ) {
        let inner = std::mem::replace(&mut self.inner, RdiCollection::new());
        self.inner = inner.keyed_singleton::<ServiceSlot<T>>(key.to_string(), move |resolver: &dyn IServiceResolver| {
            let adapter = ResolverAdapter { resolver };
            let arc_t: Arc<T> = factory(&adapter);
            Arc::new(ServiceSlot(arc_t))
        });
    }

    /// 构建 `ServiceProvider`，返回 `Arc<dyn IServiceProvider + Send + Sync>`。
    pub fn build(self) -> Arc<dyn IServiceProvider + Send + Sync> {
        let provider = self.inner.build().expect("DI build failed (circular dependency?)");
        Arc::new(crate::provider::ServiceProvider { inner: provider })
    }
}

impl Default for ServiceCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// 适配器 —— 将 `&dyn IServiceResolver`（rust-dix）适配为 `&dyn IServiceProvider`（RML）。
///
/// factory 闭包内通过 `p.get_trait::<dyn IDep>()` 解析依赖时，
/// 经此适配器委托 rust-dix resolver 的 `get_by_type_id` 完成。
struct ResolverAdapter<'a> {
    resolver: &'a dyn IServiceResolver,
}

impl<'a> IServiceProvider for ResolverAdapter<'a> {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.resolver.get_by_type_id(type_id)
    }

    fn get_keyed_service_any(
        &self,
        type_id: TypeId,
        key: &str,
    ) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.resolver.get_keyed_by_type_id(type_id, key)
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.resolver.get_by_type_id(type_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rml_core::context::ServiceProviderExt;

    trait IGreeter: Send + Sync {
        fn greet(&self) -> String;
    }

    struct EnglishGreeter;
    impl IGreeter for EnglishGreeter {
        fn greet(&self) -> String {
            "Hello".into()
        }
    }

    #[test]
    fn singleton_trait_resolution() {
        let mut s = ServiceCollection::new();
        s.add_singleton::<dyn IGreeter>(|_| Arc::new(EnglishGreeter) as Arc<dyn IGreeter>);
        let provider = s.build();
        let greeter = provider.get_trait::<dyn IGreeter>().unwrap();
        assert_eq!(greeter.greet(), "Hello");
    }

    #[test]
    fn keyed_singleton_resolution() {
        trait IProvider: Send + Sync {
            fn name(&self) -> &str;
        }
        struct A;
        struct B;
        impl IProvider for A {
            fn name(&self) -> &str {
                "a"
            }
        }
        impl IProvider for B {
            fn name(&self) -> &str {
                "b"
            }
        }

        let mut s = ServiceCollection::new();
        s.add_keyed_singleton::<dyn IProvider>("a", |_| Arc::new(A) as Arc<dyn IProvider>);
        s.add_keyed_singleton::<dyn IProvider>("b", |_| Arc::new(B) as Arc<dyn IProvider>);
        let provider = s.build();

        assert_eq!(provider.get_keyed_trait::<dyn IProvider>("a").unwrap().name(), "a");
        assert_eq!(provider.get_keyed_trait::<dyn IProvider>("b").unwrap().name(), "b");
    }

    #[test]
    fn singleton_caches() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CNT: AtomicUsize = AtomicUsize::new(0);

        trait ICounter: Send + Sync {}
        struct Counter;
        impl ICounter for Counter {}

        let mut s = ServiceCollection::new();
        s.add_singleton::<dyn ICounter>(|_| {
            CNT.fetch_add(1, Ordering::SeqCst);
            Arc::new(Counter) as Arc<dyn ICounter>
        });
        let provider = s.build();

        let _ = provider.get_trait::<dyn ICounter>();
        let _ = provider.get_trait::<dyn ICounter>();
        assert_eq!(CNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn factory_dependency_injection() {
        trait IDep: Send + Sync {
            fn value(&self) -> i32;
        }
        trait IService: Send + Sync {
            fn computed(&self) -> i32;
        }

        struct Dep;
        impl IDep for Dep {
            fn value(&self) -> i32 {
                42
            }
        }
        struct Service(Arc<dyn IDep>);
        impl IService for Service {
            fn computed(&self) -> i32 {
                self.0.value() * 2
            }
        }

        let mut s = ServiceCollection::new();
        s.add_singleton::<dyn IDep>(|_| Arc::new(Dep) as Arc<dyn IDep>);
        s.add_singleton::<dyn IService>(|p| {
            let dep = p.get_trait::<dyn IDep>().unwrap();
            Arc::new(Service(dep)) as Arc<dyn IService>
        });
        let provider = s.build();

        let svc = provider.get_trait::<dyn IService>().unwrap();
        assert_eq!(svc.computed(), 84);
    }
}

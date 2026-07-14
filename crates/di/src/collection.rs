//! ServiceCollection —— 注册容器（自维护 factory map）
//!
//! 对标 ASP.NET Core `IServiceCollection`。按 `TypeId` 索引 factory 闭包，
//! `build()` 生成 `ServiceProvider`。
//!
//! 所有服务经 `ServiceSlot<T>` 桥接存储，使 trait object（`dyn Trait`）可通过
//! `IServiceProvider` 的类型擦除层注册/查询。查询时经 `ServiceProviderExt::get_trait`
//! 还原为 `Arc<T>`。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use rml_core::context::{IServiceProvider, ServiceSlot};

/// Factory 闭包 —— 接收 `&dyn IServiceProvider`（支持 factory 内依赖注入），返回类型擦除的 `Arc`。
/// `Send + Sync` 约束确保 `ServiceProvider` 可作为 `Arc<dyn IServiceProvider + Send + Sync>` 存储。
pub(crate) type FactoryFn =
    Box<dyn Fn(&dyn IServiceProvider) -> Arc<dyn Any + Send + Sync> + Send + Sync>;

/// 服务注册容器 —— 按 `TypeId` 索引 factory 闭包。
///
/// 所有服务经 `ServiceSlot<T>` 桥接：`add_singleton::<dyn ITrait>(factory)` 注册时，
/// factory 返回的 `Arc<dyn ITrait>` 被包装为 `ServiceSlot<dyn ITrait>`（Sized），
/// 存入 `factories[TypeId::of::<ServiceSlot<dyn ITrait>>()]`。
/// 查询时经 `get_trait::<dyn ITrait>()` 还原。
pub struct ServiceCollection {
    factories: HashMap<TypeId, FactoryFn>,
    keyed_factories: HashMap<(TypeId, String), FactoryFn>,
}

impl ServiceCollection {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            keyed_factories: HashMap::new(),
        }
    }

    /// 注册单例服务。`factory` 接收 `&dyn IServiceProvider` 支持工厂内依赖注入。
    ///
    /// 服务经 `ServiceSlot<T>` 桥接存储，查询时经 `get_trait::<T>()` 还原。
    pub fn add_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        factory: impl Fn(&dyn IServiceProvider) -> Arc<T> + Send + Sync + 'static,
    ) {
        let f: FactoryFn = Box::new(move |p: &dyn IServiceProvider| {
            let arc_t: Arc<T> = factory(p);
            Arc::new(ServiceSlot(arc_t)) as Arc<dyn Any + Send + Sync>
        });
        self.factories.insert(TypeId::of::<ServiceSlot<T>>(), f);
    }

    /// 注册 keyed 单例服务。`key` 区分同一 trait 的多个实现。
    pub fn add_keyed_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        key: &str,
        factory: impl Fn(&dyn IServiceProvider) -> Arc<T> + Send + Sync + 'static,
    ) {
        let f: FactoryFn = Box::new(move |p: &dyn IServiceProvider| {
            Arc::new(ServiceSlot(factory(p))) as Arc<dyn Any + Send + Sync>
        });
        self.keyed_factories
            .insert((TypeId::of::<ServiceSlot<T>>(), key.to_string()), f);
    }

    /// 构建 `ServiceProvider`，返回 `Arc<dyn IServiceProvider + Send + Sync>`。
    pub fn build(self) -> Arc<dyn IServiceProvider + Send + Sync> {
        Arc::new(crate::provider::ServiceProvider {
            factories: self.factories,
            keyed_factories: self.keyed_factories,
            cache: std::sync::RwLock::new(HashMap::new()),
            keyed_cache: std::sync::RwLock::new(HashMap::new()),
        })
    }
}

impl Default for ServiceCollection {
    fn default() -> Self {
        Self::new()
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

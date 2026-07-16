//! 贡献注册表内部实现
//!
//! 框架内部：桥接 contribute → host，按 host_id 路由 register 调用到 host 的 `IContributionHost::add`。
//! Registry 存储 `Arc<dyn IContributionHost>` trait object，经 trait 方法路由贡献，
//! 不依赖具体存储类型、不经 Entity 系统、不存闭包。
//!
//! 存储形态：`ContributionRegistry` 作为 GPUI Global 存储（`cx.set_global`），
//! 框架内部服务不经 `IServiceProvider`，与 i18n/theme 范式对齐。
//! newtype `ContributionRegistry(Arc<Inner>)` 使其 `Clone` 并可独立实现 `Global`（绕开 orphan rule）。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::Global;
use rml_core::contribution::{
    ContributionOptions, IContribution, IContributionHost, IContributionRegistry,
};

/// 框架内部实现：桥接 contribute → host
///
/// newtype 包装 `Arc<Inner>`：`Clone` 为浅拷贝（引用计数递增），并使本类型可独立实现 `Global`
/// （绕开为 `Arc<T>` 实现 trait 的 orphan rule 限制）。
#[derive(Clone)]
pub struct ContributionRegistry(Arc<ContributionRegistryInner>);

struct ContributionRegistryInner {
    hosts: RwLock<HashMap<String, Arc<dyn IContributionHost>>>,
}

/// 框架内部服务经 GPUI Global 存储（不经过 IServiceProvider）。
/// 由 `bootstrap_runtime` 经 `cx.set_global(ContributionRegistry::new())` 注入。
impl Global for ContributionRegistry {}

impl ContributionRegistry {
    pub fn new() -> Self {
        Self(Arc::new(ContributionRegistryInner {
            hosts: RwLock::new(HashMap::new()),
        }))
    }

    pub fn has_host(&self, host_id: &str) -> bool {
        self.0.hosts.read().unwrap().contains_key(host_id)
    }
}

impl Default for ContributionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl IContributionRegistry for ContributionRegistry {
    fn add(&self, host_id: &str, host: Arc<dyn IContributionHost>) {
        self.0
            .hosts
            .write()
            .unwrap()
            .insert(host_id.to_string(), host);
    }

    fn remove(&self, host_id: &str) {
        self.0.hosts.write().unwrap().remove(host_id);
    }

    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: Option<ContributionOptions>,
    ) {
        let hosts = self.0.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add(contribution, options);
        }
        // host 未注册时贡献丢弃。要求 host 在 on_loaded 中先 register_host 注册自身（或共享存储），
        // 再调用 bootstrap_host_contributions(cx, host_id) 触发该 host_id 的贡献注册。
    }

    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool {
        let hosts = self.0.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.remove(contribution_id);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_has_no_host() {
        let registry = ContributionRegistry::new();
        assert!(!registry.has_host("test.host"));
    }
}

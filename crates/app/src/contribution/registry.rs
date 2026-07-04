//! 贡献注册表内部实现
//!
//! 框架内部：桥接 contribute → host，按 host_id 路由 register 调用到 host 的 `IContributionHost::add`。
//! Registry 存储 `Arc<dyn IContributionHost>` trait object，经 trait 方法路由贡献，
//! 不依赖具体存储类型、不经 Entity 系统、不存闭包。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rml_core::contribution::{
    ContributionOptions, IContribution, IContributionHost, IContributionRegistry,
};

/// 框架内部实现：桥接 contribute → host
pub struct ContributionRegistry {
    hosts: RwLock<HashMap<String, Arc<dyn IContributionHost>>>,
}

impl ContributionRegistry {
    pub fn new() -> Self {
        Self {
            hosts: RwLock::new(HashMap::new()),
        }
    }

    pub fn has_host(&self, host_id: &str) -> bool {
        self.hosts.read().unwrap().contains_key(host_id)
    }
}

impl Default for ContributionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl IContributionRegistry for ContributionRegistry {
    fn add(&self, host_id: &str, host: Arc<dyn IContributionHost>) {
        self.hosts
            .write()
            .unwrap()
            .insert(host_id.to_string(), host);
    }

    fn remove(&self, host_id: &str) {
        self.hosts.write().unwrap().remove(host_id);
    }

    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: Option<ContributionOptions>,
    ) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add(contribution, options);
        }
        // host 未注册时贡献丢弃。要求 host 在 on_loaded 中先 register_host 注册自身（或共享存储），
        // 再调用 bootstrap_host_contributions(cx, host_id) 触发该 host_id 的贡献注册。
    }

    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool {
        let hosts = self.hosts.read().unwrap();
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

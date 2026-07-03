//! 贡献注册表内部实现
//!
//! 框架内部：桥接 contribute → host，按 host_id 路由 register/register_visual 调用到 host.add/add_visual。
//! Registry 仅存储 `IContributionHost`，不存储贡献本身。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rml_core::command::ICommand;
use rml_core::contribution::{
    ContributionOptions, IContribution, IContributionHost, IContributionRegistry,
    IVisualContribution,
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
    fn add_host(&self, host: Arc<dyn IContributionHost>) {
        let id = host.id().to_string();
        self.hosts.write().unwrap().insert(id, host);
    }

    fn remove_host(&self, host_id: &str) {
        self.hosts.write().unwrap().remove(host_id);
    }

    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
    ) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add(contribution, options);
        } else {
            // host 未注册时贡献被丢弃。要求 host 在 on_loaded 中先 __rml_install_host 注册自身，
            // 再触发该 host_id 的贡献注册（由 __rml_install_host 内部同步完成）。
            let _ = (host_id, contribution, options);
        }
    }

    fn register_visual(
        &self,
        host_id: &str,
        contribution: Arc<dyn IVisualContribution>,
        options: ContributionOptions,
    ) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add_visual(contribution, options);
        } else {
            let _ = (host_id, contribution, options);
        }
    }

    fn register_command(
        &self,
        host_id: &str,
        command: Arc<dyn ICommand>,
        options: ContributionOptions,
    ) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add_command(command, options);
        } else {
            let _ = (host_id, command, options);
        }
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

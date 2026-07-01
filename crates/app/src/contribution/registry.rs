//! 贡献注册表内部实现（通过 [`ContributionExt`](super::global::ContributionExt) 访问）

use std::collections::HashMap;
use std::sync::Arc;

use gpui::App;
use rml_core::contribution::{
    ComponentEntityCache, ContributedEntry, ContributionOptions, IContribution,
};

use super::entry::{add_entry, data_entry_dyn};
use super::host::ContributionHost;
use super::registerable::Registerable;

type HostListener = Box<dyn Fn(&mut App) + Send + Sync>;

/// 全局贡献注册表（框架内部；应用通过 `App` 扩展 trait 操作）
pub struct ContributionRegistry {
    hosts: HashMap<String, ContributionHost>,
    entity_cache: rml_core::contribution_cache::ComponentEntityCacheImpl,
    listeners: HashMap<String, Vec<HostListener>>,
}

impl ContributionRegistry {
    pub fn new() -> Self {
        Self {
            hosts: HashMap::new(),
            entity_cache: rml_core::contribution_cache::ComponentEntityCacheImpl::new(),
            listeners: HashMap::new(),
        }
    }

    pub fn entity_cache_mut(&mut self) -> &mut rml_core::contribution_cache::ComponentEntityCacheImpl {
        &mut self.entity_cache
    }

    pub fn ensure_host(&mut self, host_id: impl Into<String>) {
        let id = host_id.into();
        self.hosts
            .entry(id.clone())
            .or_insert_with(|| ContributionHost::new(id));
    }

    pub fn remove_host(&mut self, host_id: &str) {
        if let Some(host) = self.hosts.remove(host_id) {
            for entry in host.entries() {
                self.entity_cache.clear(entry.contribution.id());
            }
        }
        self.listeners.remove(host_id);
    }

    pub fn entries(&self, host_id: &str) -> &[ContributedEntry] {
        self.hosts
            .get(host_id)
            .map(ContributionHost::entries)
            .unwrap_or(&[])
    }

    pub fn revision(&self, host_id: &str) -> u64 {
        self.hosts
            .get(host_id)
            .map(ContributionHost::revision)
            .unwrap_or(0)
    }

    pub fn subscribe_host(&mut self, host_id: &str, listener: HostListener) {
        self.listeners
            .entry(host_id.to_string())
            .or_default()
            .push(listener);
    }

    fn notify_host(&self, host_id: &str, cx: &mut App) {
        if let Some(listeners) = self.listeners.get(host_id) {
            for listener in listeners {
                listener(cx);
            }
        }
    }

    pub fn register_typed<T>(
        &mut self,
        host_id: &str,
        contribution: Arc<T>,
        options: ContributionOptions,
        cx: &mut App,
    ) where
        T: Registerable + 'static,
    {
        let entry = T::into_entry(contribution, options);
        self.register_entry(host_id, entry, cx);
    }

    pub fn register_dyn(
        &mut self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
        cx: &mut App,
    ) {
        let entry = data_entry_dyn(contribution, options);
        self.register_entry(host_id, entry, cx);
    }

    pub fn register_entry(&mut self, host_id: &str, entry: ContributedEntry, cx: &mut App) {
        self.ensure_host(host_id);
        add_entry(&mut self.hosts, host_id, entry, cx);
        self.notify_host(host_id, cx);
    }

    pub fn unregister(&mut self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool {
        let removed = self
            .hosts
            .get_mut(host_id)
            .map(|h| h.remove(contribution_id, cx))
            .unwrap_or(false);
        if removed {
            self.entity_cache.clear(contribution_id);
            self.notify_host(host_id, cx);
        }
        removed
    }
}

impl Default for ContributionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ContributionRegistry;

    #[test]
    fn ensure_host_creates_empty_slot() {
        let mut registry = ContributionRegistry::new();
        registry.ensure_host("test.host");
        assert!(registry.entries("test.host").is_empty());
        assert_eq!(registry.revision("test.host"), 0);
    }

    #[test]
    fn remove_host_clears_entries() {
        let mut registry = ContributionRegistry::new();
        registry.ensure_host("h");
        registry.remove_host("h");
        assert!(registry.entries("h").is_empty());
    }
}

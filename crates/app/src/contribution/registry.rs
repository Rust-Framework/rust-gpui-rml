//! `IContributionRegistry` 实现 + 统一 `register`

use std::collections::HashMap;
use std::sync::Arc;

use gpui::App;
use rml_core::contribution::{
    ComponentEntityCache, ContributedEntry, ContributionOptions, IContributionHost,
    IContributionRegistry,
};

use super::entry::{add_entry, data_entry_dyn};
use super::host::ContributionHost;
use super::registerable::Registerable;

/// 全局贡献注册表
pub struct ContributionRegistry {
    hosts: HashMap<String, Box<dyn IContributionHost>>,
    entity_cache: rml_core::contribution_cache::ComponentEntityCacheImpl,
}

impl ContributionRegistry {
    pub fn new() -> Self {
        Self {
            hosts: HashMap::new(),
            entity_cache: rml_core::contribution_cache::ComponentEntityCacheImpl::new(),
        }
    }

    pub fn entity_cache_mut(&mut self) -> &mut rml_core::contribution_cache::ComponentEntityCacheImpl {
        &mut self.entity_cache
    }

    /// 确保 host 存在（首次向某 `host_id` 注册时自动调用；应用也可提前创建以绑定 `on_changed`）
    pub fn ensure_host(&mut self, host_id: impl Into<String>) {
        let id = host_id.into();
        if !self.hosts.contains_key(&id) {
            self.add_host(Box::new(ContributionHost::new(id)));
        }
    }

    /// 为指定 host 设置变更回调（供 Shell 驱动 UI 同步）
    pub fn set_host_on_changed(
        &mut self,
        host_id: &str,
        callback: Box<dyn Fn(&mut App) + Send + Sync>,
    ) {
        if let Some(host) = self.hosts.get_mut(host_id) {
            host.set_on_changed(callback);
        }
    }

    /// 统一注册入口：由 `Registerable::into_entry` 区分数据/视图贡献
    pub fn register<T>(
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

    /// 直接注册已构建条目（host 不存在时自动创建）
    pub fn register_entry(&mut self, host_id: &str, entry: ContributedEntry, cx: &mut App) {
        self.ensure_host(host_id);
        add_entry(&mut self.hosts, host_id, entry, cx);
    }
}

impl IContributionRegistry for ContributionRegistry {
    fn add_host(&mut self, host: Box<dyn IContributionHost>) {
        let id = host.host_id().to_string();
        self.hosts.insert(id, host);
    }

    fn host(&self, host_id: &str) -> Option<&dyn IContributionHost> {
        self.hosts.get(host_id).map(|h| h.as_ref())
    }

    /// 动态 trait 对象注册
    fn register(
        &mut self,
        host_id: &str,
        contribution: Arc<dyn rml_core::contribution::IContribution>,
        options: ContributionOptions,
        cx: &mut App,
    ) {
        let entry = data_entry_dyn(contribution, options);
        self.register_entry(host_id, entry, cx);
    }

    fn unregister(&mut self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool {
        let removed = self
            .hosts
            .get_mut(host_id)
            .map(|h| h.remove(contribution_id, cx))
            .unwrap_or(false);
        if removed {
            self.entity_cache.clear(contribution_id);
        }
        removed
    }
}

impl Default for ContributionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

//! 贡献注册表内部实现 + 视觉提取器进程级静态表
//!
//! 框架内部：桥接 contribute → host，按 host_id 路由 register 调用到 host.add。
/// 视觉提取器由 `#[contribute]` 宏在 `#[ctor::ctor]` 阶段写入进程级静态表。

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use rml_core::contribution::{
    ContributionOptions, IContribution, IContributionHost, IContributionRegistry,
};

/// 进程级视觉提取器表——由 `#[contribute]` 宏生成的 `#[ctor::ctor]` 在进程启动期写入。
static VISUAL_EXTRACTORS: OnceLock<RwLock<HashMap<TypeId, rml_core::contribution::VisualExtractor>>> =
    OnceLock::new();

fn visual_extractors() -> &'static RwLock<HashMap<TypeId, rml_core::contribution::VisualExtractor>> {
    VISUAL_EXTRACTORS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// `#[contribute]` 宏在 `#[ctor::ctor]` 中调用：注册视觉提取器。
#[doc(hidden)]
pub fn register_visual_extractor(type_id: TypeId, extractor: rml_core::contribution::VisualExtractor) {
    visual_extractors().write().unwrap().insert(type_id, extractor);
}

/// host 在 `add` 内调用：按 `TypeId` 查找提取器，返回 `Arc<dyn IVisualContribution>`。
pub fn extract_visual(
    contribution: &Arc<dyn IContribution>,
) -> Option<Arc<dyn rml_core::contribution::IVisualContribution>> {
    let type_id = (**contribution).type_id();
    let extractors = visual_extractors().read().unwrap();
    extractors.get(&type_id).and_then(|f| f(contribution))
}

/// 框架内部实现：桥接 contribute → host
pub struct ContributionRegistry {
    hosts: RwLock<HashMap<String, Arc<dyn IContributionHost>>>,
    pending: RwLock<HashMap<String, Vec<(Arc<dyn IContribution>, ContributionOptions)>>>,
}

impl ContributionRegistry {
    pub fn new() -> Self {
        Self {
            hosts: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashMap::new()),
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
    fn add(&self, host: Arc<dyn IContributionHost>) {
        let id = host.id().to_string();
        {
            let mut hosts = self.hosts.write().unwrap();
            hosts.insert(id.clone(), host);
        }

        // 重放 pending 队列 —— 直接调 host.add()，无需 cx
        let queue = {
            let mut pending = self.pending.write().unwrap();
            pending.remove(&id).unwrap_or_default()
        };

        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(&id) {
            for (contribution, options) in queue {
                host.add(contribution, options);
            }
        }
    }

    fn remove(&self, host_id: &str) {
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
            drop(hosts);
            self.pending
                .write()
                .unwrap()
                .entry(host_id.to_string())
                .or_default()
                .push((contribution, options));
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

    fn take_pending(&self, host_id: &str) -> Vec<(Arc<dyn IContribution>, ContributionOptions)> {
        let mut pending = self.pending.write().unwrap();
        pending.remove(host_id).unwrap_or_default()
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

//! `IContributionHost` 管理器实现
//!
//! 管理贡献集合与变更同步；**不**负责 UI 呈现。

use std::sync::atomic::{AtomicU64, Ordering};

use gpui::App;
use rml_core::contribution::{ContributedEntry, IContributionHost};

/// 默认贡献点主机
pub struct ContributionHost {
    host_id: String,
    entries: Vec<ContributedEntry>,
    version: AtomicU64,
    on_changed: Option<Box<dyn Fn(&mut App) + Send + Sync>>,
}

impl ContributionHost {
    pub fn new(host_id: impl Into<String>) -> Self {
        Self {
            host_id: host_id.into(),
            entries: Vec::new(),
            version: AtomicU64::new(0),
            on_changed: None,
        }
    }

    fn bump(&self, cx: &mut App) {
        self.version.fetch_add(1, Ordering::SeqCst);
        if let Some(cb) = &self.on_changed {
            cb(cx);
        }
    }

    fn sort_entries(&mut self) {
        self.entries
            .sort_by(|a, b| a.options.order.cmp(&b.options.order));
    }
}

impl IContributionHost for ContributionHost {
    fn host_id(&self) -> &str {
        &self.host_id
    }

    fn add(&mut self, entry: ContributedEntry, cx: &mut App) {
        let id = entry.contribution.id().to_string();
        self.entries.retain(|e| e.contribution.id() != id);
        entry.contribution.on_register(&self.host_id, cx);
        self.entries.push(entry);
        self.sort_entries();
        self.bump(cx);
    }

    fn remove(&mut self, contribution_id: &str, cx: &mut App) -> bool {
        let before = self.entries.len();
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| e.contribution.id() == contribution_id)
        {
            entry.contribution.on_unregister(&self.host_id, cx);
        }
        self.entries
            .retain(|e| e.contribution.id() != contribution_id);
        let removed = self.entries.len() < before;
        if removed {
            self.bump(cx);
        }
        removed
    }

    fn entries(&self) -> &[ContributedEntry] {
        &self.entries
    }

    fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    fn set_on_changed(&mut self, callback: Box<dyn Fn(&mut App) + Send + Sync>) {
        self.on_changed = Some(callback);
    }
}

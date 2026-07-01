//! `IContributionHost` 管理器实现
//!
//! 管理贡献集合；**不**负责 UI 呈现或变更通知（由 Registry 统一派发）。

use std::sync::atomic::{AtomicU64, Ordering};

use gpui::App;
use rml_core::contribution::{ContributedEntry, IContributionHost};

/// 默认贡献点主机
pub struct ContributionHost {
    id: String,
    entries: Vec<ContributedEntry>,
    revision: AtomicU64,
}

impl ContributionHost {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            revision: AtomicU64::new(0),
        }
    }

    pub fn entries(&self) -> &[ContributedEntry] {
        &self.entries
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    fn bump_revision(&self) {
        self.revision.fetch_add(1, Ordering::SeqCst);
    }

    fn sort_entries(&mut self) {
        self.entries
            .sort_by(|a, b| a.options.order.cmp(&b.options.order));
    }
}

/// 构造类型擦除的 host 实例（`#[contributehost]` 生成代码专用）。
#[doc(hidden)]
pub fn box_host(id: impl Into<String>) -> Box<dyn IContributionHost> {
    Box::new(ContributionHost::new(id))
}

impl IContributionHost for ContributionHost {
    fn id(&self) -> &str {
        &self.id
    }

    fn add(&mut self, entry: ContributedEntry, cx: &mut App) {
        let id = entry.contribution.id().to_string();
        self.entries.retain(|e| e.contribution.id() != id);
        entry.contribution.on_register(self.id(), cx);
        self.entries.push(entry);
        self.sort_entries();
        self.bump_revision();
    }

    fn remove(&mut self, contribution_id: &str, cx: &mut App) -> bool {
        let before = self.entries.len();
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| e.contribution.id() == contribution_id)
        {
            entry.contribution.on_unregister(self.id(), cx);
        }
        self.entries
            .retain(|e| e.contribution.id() != contribution_id);
        let removed = self.entries.len() < before;
        if removed {
            self.bump_revision();
        }
        removed
    }
}

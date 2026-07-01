//! 贡献点主机存储（内部实现，非公开扩展点）

use std::sync::atomic::{AtomicU64, Ordering};

use gpui::App;
use rml_core::contribution::ContributedEntry;

/// 单个 host id 下的贡献条目集合
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

    pub fn id(&self) -> &str {
        &self.id
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

    pub fn add(&mut self, entry: ContributedEntry, _cx: &mut App) {
        let id = entry.contribution.id().to_string();
        self.entries.retain(|e| e.contribution.id() != id);
        self.entries.push(entry);
        self.sort_entries();
        self.bump_revision();
    }

    pub fn remove(&mut self, contribution_id: &str, _cx: &mut App) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|e| e.contribution.id() != contribution_id);
        let removed = self.entries.len() < before;
        if removed {
            self.bump_revision();
        }
        removed
    }
}

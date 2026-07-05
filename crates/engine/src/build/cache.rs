//! 增量缓存（JSON）
//!
//! 记录每个 `.rml` 文件的 sha256 哈希，未变化则跳过重新生成。
//! 详见文档 §10.4.5 增量编译。
//!
//! ## 失效策略
//!
//! 仅靠 `.rml` 文件哈希不够：
//! - 当 engine crate 的 codegen/parser/tags 等实现变化时，即使 `.rml` 源不变，
//!   也需要重新生成。因此 Cache 额外记录 `engine_hash`（engine 源码哈希），
//!   加载时若与当前不匹配，则视为全部过期。
//! - 当 `.rml.rs` code-behind 文件变化（如新增 `#[computed]` 方法）时，
//!   codegen 上下文（computed_methods）改变，需重新生成。Cache 额外记录
//!   `codebehind_hash`，逐文件比对，不匹配则该文件重新生成。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
pub struct Cache {
    /// `.rml` 文件路径（字符串） → sha256 hex（源码哈希）
    pub entries: HashMap<String, String>,
    /// 上次构建时 engine crate 的源码哈希；不匹配则 entries 全部失效
    #[serde(default)]
    pub engine_hash: Option<String>,
    /// `.rml` 文件路径 → 对应 `.rml.rs` code-behind 的 sha256 hex；
    /// 不匹配则该文件重新生成（即使 .rml 源未变）。
    #[serde(default)]
    pub codebehind_hash: HashMap<String, String>,
}

impl Cache {
    /// 从 JSON 文件加载；文件不存在或解析失败时返回空缓存。
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// 写回 JSON 文件。
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let s = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(path, s)
    }

    /// 判断缓存对当前 engine 哈希是否有效。
    ///
    /// - `cache.engine_hash` 为 None：旧版本缓存，视为失效
    /// - `cache.engine_hash` 与传入哈希不等：engine 源码变化，失效
    /// - 相等：可保留 entries
    pub fn is_valid_for_engine(&self, current_engine_hash: &str) -> bool {
        match &self.engine_hash {
            Some(h) => h == current_engine_hash,
            None => false,
        }
    }

    /// 标记缓存为当前 engine 哈希对应。
    pub fn stamp_engine(&mut self, current_engine_hash: String) {
        self.engine_hash = Some(current_engine_hash);
    }

    /// 清空所有 entries（用于 engine 变化时强制全部重新生成）。
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.codebehind_hash.clear();
    }

    /// 判断单个 `.rml` 文件的 code-behind（`.rml.rs`）是否与缓存中记录的哈希一致。
    ///
    /// 返回 true 表示 code-behind 未变化（可跳过重新生成）；
    /// 返回 false 表示 code-behind 已变化或缓存中无记录（必须重新生成）。
    pub fn is_codebehind_unchanged(&self, rml_key: &str, current_cb_hash: &str) -> bool {
        match self.codebehind_hash.get(rml_key) {
            Some(h) => h == current_cb_hash,
            None => false,
        }
    }

    /// 记录 `.rml` 文件对应的 `.rml.rs` 哈希。
    pub fn stamp_codebehind(&mut self, rml_key: String, cb_hash: String) {
        self.codebehind_hash.insert(rml_key, cb_hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 全局递增计数器，确保每个测试使用独立的临时文件路径。
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// 构造唯一的临时文件路径（不创建文件），用于 load/save 测试。
    fn unique_temp_path() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "rml_cache_test_{}_{}.json",
            std::process::id(),
            id
        ))
    }

    /// 测试结束后清理临时文件。
    struct TempFileGuard(PathBuf);
    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    // ─── load：文件不存在时返回空缓存 ───

    #[test]
    fn load_missing_file_returns_empty() {
        let path = unique_temp_path();
        let _guard = TempFileGuard(path.clone());
        let cache = Cache::load(&path);
        assert!(cache.entries.is_empty());
        assert!(cache.engine_hash.is_none());
        assert!(cache.codebehind_hash.is_empty());
    }

    // ─── load：解析失败时返回空缓存 ───

    #[test]
    fn load_corrupt_json_returns_empty() {
        let path = unique_temp_path();
        let _guard = TempFileGuard(path.clone());
        std::fs::write(&path, "{ not valid json").unwrap();
        let cache = Cache::load(&path);
        assert!(cache.entries.is_empty(), "corrupt JSON should yield empty cache");
    }

    // ─── save/load：往返一致性 ───

    #[test]
    fn save_and_load_roundtrip_preserves_entries() {
        let path = unique_temp_path();
        let _guard = TempFileGuard(path.clone());

        let mut original = Cache::default();
        original.entries.insert(
            "src/foo.rml".to_string(),
            "abc123".to_string(),
        );
        original.entries.insert(
            "src/bar.rml".to_string(),
            "def456".to_string(),
        );

        original.save(&path).unwrap();
        let loaded = Cache::load(&path);

        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries.get("src/foo.rml").unwrap(), "abc123");
        assert_eq!(loaded.entries.get("src/bar.rml").unwrap(), "def456");
    }

    #[test]
    fn save_and_load_roundtrip_preserves_engine_hash() {
        let path = unique_temp_path();
        let _guard = TempFileGuard(path.clone());

        let mut original = Cache::default();
        original.stamp_engine("engine_v1_hash".to_string());

        original.save(&path).unwrap();
        let loaded = Cache::load(&path);

        assert_eq!(loaded.engine_hash.as_deref(), Some("engine_v1_hash"));
    }

    #[test]
    fn save_and_load_roundtrip_preserves_codebehind_hash() {
        let path = unique_temp_path();
        let _guard = TempFileGuard(path.clone());

        let mut original = Cache::default();
        original.stamp_codebehind("src/foo.rml".to_string(), "cb_hash_1".to_string());
        original.stamp_codebehind("src/bar.rml".to_string(), "cb_hash_2".to_string());

        original.save(&path).unwrap();
        let loaded = Cache::load(&path);

        assert_eq!(loaded.codebehind_hash.len(), 2);
        assert_eq!(loaded.codebehind_hash.get("src/foo.rml").unwrap(), "cb_hash_1");
        assert_eq!(loaded.codebehind_hash.get("src/bar.rml").unwrap(), "cb_hash_2");
    }

    // ─── is_valid_for_engine ───

    #[test]
    fn is_valid_for_engine_none_hash_returns_false() {
        // engine_hash 为 None（旧版缓存）→ 视为失效
        let cache = Cache::default();
        assert!(!cache.is_valid_for_engine("any_hash"));
    }

    #[test]
    fn is_valid_for_engine_matching_hash_returns_true() {
        let mut cache = Cache::default();
        cache.stamp_engine("engine_v1".to_string());
        assert!(cache.is_valid_for_engine("engine_v1"));
    }

    #[test]
    fn is_valid_for_engine_mismatched_hash_returns_false() {
        let mut cache = Cache::default();
        cache.stamp_engine("engine_v1".to_string());
        assert!(!cache.is_valid_for_engine("engine_v2"));
    }

    // ─── stamp_engine ───

    #[test]
    fn stamp_engine_sets_hash() {
        let mut cache = Cache::default();
        assert!(cache.engine_hash.is_none());
        cache.stamp_engine("new_hash".to_string());
        assert_eq!(cache.engine_hash.as_deref(), Some("new_hash"));
    }

    #[test]
    fn stamp_engine_overwrites_previous_hash() {
        let mut cache = Cache::default();
        cache.stamp_engine("v1".to_string());
        cache.stamp_engine("v2".to_string());
        assert_eq!(cache.engine_hash.as_deref(), Some("v2"));
    }

    // ─── invalidate_all ───

    #[test]
    fn invalidate_all_clears_entries_and_codebehind_hash() {
        let mut cache = Cache::default();
        cache.entries.insert("a.rml".to_string(), "hash_a".to_string());
        cache.entries.insert("b.rml".to_string(), "hash_b".to_string());
        cache.stamp_codebehind("a.rml".to_string(), "cb_a".to_string());
        cache.stamp_engine("engine_v1".to_string());

        cache.invalidate_all();

        assert!(cache.entries.is_empty(), "entries should be cleared");
        assert!(
            cache.codebehind_hash.is_empty(),
            "codebehind_hash should be cleared"
        );
        // engine_hash 不应被清空（用于后续 is_valid_for_engine 检查）
        assert_eq!(cache.engine_hash.as_deref(), Some("engine_v1"));
    }

    #[test]
    fn invalidate_all_on_empty_cache_is_noop() {
        let mut cache = Cache::default();
        cache.invalidate_all();
        assert!(cache.entries.is_empty());
        assert!(cache.codebehind_hash.is_empty());
    }

    // ─── is_codebehind_unchanged ───

    #[test]
    fn is_codebehind_unchanged_missing_key_returns_false() {
        // 缓存中无该 .rml 文件的 codebehind 记录 → 视为已变化（必须重新生成）
        let cache = Cache::default();
        assert!(!cache.is_codebehind_unchanged("src/missing.rml", "any_hash"));
    }

    #[test]
    fn is_codebehind_unchanged_matching_hash_returns_true() {
        let mut cache = Cache::default();
        cache.stamp_codebehind("src/foo.rml".to_string(), "cb_v1".to_string());
        assert!(cache.is_codebehind_unchanged("src/foo.rml", "cb_v1"));
    }

    #[test]
    fn is_codebehind_unchanged_mismatched_hash_returns_false() {
        let mut cache = Cache::default();
        cache.stamp_codebehind("src/foo.rml".to_string(), "cb_v1".to_string());
        assert!(!cache.is_codebehind_unchanged("src/foo.rml", "cb_v2"));
    }

    // ─── stamp_codebehind ───

    #[test]
    fn stamp_codebehind_inserts_new_entry() {
        let mut cache = Cache::default();
        assert!(cache.codebehind_hash.is_empty());
        cache.stamp_codebehind("src/foo.rml".to_string(), "hash_1".to_string());
        assert_eq!(cache.codebehind_hash.len(), 1);
        assert_eq!(cache.codebehind_hash.get("src/foo.rml").unwrap(), "hash_1");
    }

    #[test]
    fn stamp_codebehind_overwrites_existing_entry() {
        let mut cache = Cache::default();
        cache.stamp_codebehind("src/foo.rml".to_string(), "v1".to_string());
        cache.stamp_codebehind("src/foo.rml".to_string(), "v2".to_string());
        assert_eq!(cache.codebehind_hash.len(), 1);
        assert_eq!(cache.codebehind_hash.get("src/foo.rml").unwrap(), "v2");
    }

    #[test]
    fn stamp_codebehind_multiple_entries_coexist() {
        let mut cache = Cache::default();
        cache.stamp_codebehind("a.rml".to_string(), "hash_a".to_string());
        cache.stamp_codebehind("b.rml".to_string(), "hash_b".to_string());
        cache.stamp_codebehind("c.rml".to_string(), "hash_c".to_string());
        assert_eq!(cache.codebehind_hash.len(), 3);
    }

    // ─── 综合场景：增量缓存工作流 ───

    #[test]
    fn incremental_workflow_engine_changed_invalidates_all() {
        // 场景：第一次构建后，engine 源码变化 → is_valid_for_engine 返回 false，
        // 调用 invalidate_all 清空 entries，但保留 engine_hash（已由 stamp_engine 更新）
        let mut cache = Cache::default();
        cache.entries.insert("a.rml".to_string(), "h1".to_string());
        cache.entries.insert("b.rml".to_string(), "h2".to_string());
        cache.stamp_codebehind("a.rml".to_string(), "cb1".to_string());
        cache.stamp_engine("engine_v1".to_string());

        // engine 变化
        assert!(!cache.is_valid_for_engine("engine_v2"));
        cache.invalidate_all();
        cache.stamp_engine("engine_v2".to_string());

        assert!(cache.entries.is_empty());
        assert!(cache.codebehind_hash.is_empty());
        assert_eq!(cache.engine_hash.as_deref(), Some("engine_v2"));
    }

    #[test]
    fn incremental_workflow_codebehind_changed_only_affects_one_file() {
        // 场景：单个 .rml.rs 文件变化，仅该文件需重新生成
        let mut cache = Cache::default();
        cache.entries.insert("a.rml".to_string(), "h1".to_string());
        cache.entries.insert("b.rml".to_string(), "h2".to_string());
        cache.stamp_codebehind("a.rml".to_string(), "cb_v1".to_string());
        cache.stamp_codebehind("b.rml".to_string(), "cb_v1".to_string());
        cache.stamp_engine("engine_v1".to_string());

        // engine 未变化
        assert!(cache.is_valid_for_engine("engine_v1"));

        // a.rml.rs 变化（b.rml.rs 未变）
        assert!(!cache.is_codebehind_unchanged("a.rml", "cb_v2"));
        assert!(cache.is_codebehind_unchanged("b.rml", "cb_v1"));

        // 更新 a.rml.rs 的哈希
        cache.stamp_codebehind("a.rml".to_string(), "cb_v2".to_string());
        assert!(cache.is_codebehind_unchanged("a.rml", "cb_v2"));
    }

    #[test]
    fn empty_cache_save_load_roundtrip() {
        // 空 Cache 也应能正确序列化/反序列化
        let path = unique_temp_path();
        let _guard = TempFileGuard(path.clone());

        let original = Cache::default();
        original.save(&path).unwrap();
        let loaded = Cache::load(&path);

        assert!(loaded.entries.is_empty());
        assert!(loaded.engine_hash.is_none());
        assert!(loaded.codebehind_hash.is_empty());
    }
}


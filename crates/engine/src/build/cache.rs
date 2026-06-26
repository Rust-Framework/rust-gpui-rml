//! 增量缓存（JSON）
//!
//! 记录每个 `.rml` 文件的 sha256 哈希，未变化则跳过重新生成。
//! 详见文档 §10.4.5 增量编译。
//!
//! ## 失效策略
//!
//! 仅靠 `.rml` 文件哈希不够：当 engine crate 的 codegen/parser/tags 等实现变化时，
//! 即使 `.rml` 源不变，也需要重新生成。因此 Cache 额外记录 `engine_hash`（engine
//! 源码哈希），加载时若与当前不匹配，则视为全部过期。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
pub struct Cache {
    /// 文件路径（字符串） → sha256 hex
    pub entries: HashMap<String, String>,
    /// 上次构建时 engine crate 的源码哈希；不匹配则 entries 全部失效
    #[serde(default)]
    pub engine_hash: Option<String>,
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
        let s = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
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
    }
}

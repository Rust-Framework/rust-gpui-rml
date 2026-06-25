//! 增量缓存（JSON）
//!
//! 记录每个 `.rml` 文件的 sha256 哈希，未变化则跳过重新生成。
//! 详见文档 §10.4.5 增量编译。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
pub struct Cache {
    /// 文件路径（字符串） → sha256 hex
    pub entries: HashMap<String, String>,
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
}

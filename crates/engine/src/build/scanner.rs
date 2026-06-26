//! 递归扫描 `.rml` 文件
//!
//! 使用 `walkdir` 递归遍历扫描目录，过滤扩展名为 `.rml` 的文件。

use std::path::PathBuf;
use walkdir::WalkDir;

/// 在给定目录列表中递归扫描所有 `.rml` 文件，返回排序后的路径列表。
///
/// 不存在的目录会被静默跳过（避免可选模板目录报错）。
pub fn scan(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rml") {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    files
}

//! 扫描 demo/src 目录构建文件树（Vec<TreeItem>）。

use std::path::Path;

use gpui::SharedString;
use rml_ui::TreeItem;

/// 扫描 `demo/src/` 目录递归构建文件树。
///
/// `id` 用相对 `src/` 的路径（如 `cases/button_case.rml.rs`），用于后续打开文件。
/// 文件夹优先，然后按名称排序。跳过 `target/`、`.git/` 等目录。
///
/// 路径解析：编译时 `CARGO_MANIFEST_DIR` 指向 `demo/`，运行时可靠定位 `demo/src`，
/// 不依赖 `current_dir()`（运行时 cwd 可能是 exe 目录或任意工作目录）。
pub fn build_source_tree() -> Vec<TreeItem> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = std::path::Path::new(manifest_dir).join("src");
    if !src_dir.exists() {
        return Vec::new();
    }
    scan_dir(&src_dir, "")
}

fn scan_dir(dir: &Path, prefix: &str) -> Vec<TreeItem> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut folders: Vec<(String, std::path::PathBuf)> = Vec::new();
    let mut files: Vec<(String, std::path::PathBuf)> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if name.starts_with('.') {
            continue;
        }

        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };

        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            folders.push((relative, path));
        } else if path.is_file() {
            if !is_source_file(&name) {
                continue;
            }
            files.push((relative, path));
        }
    }

    folders.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    folders
        .into_iter()
        .map(|(relative, path)| {
            let label: SharedString = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&relative)
                .to_string()
                .into();
            let children = scan_dir(&path, &relative);
            TreeItem::new(relative, label).children(children)
        })
        .chain(files.into_iter().map(|(relative, path)| {
            let label: SharedString = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&relative)
                .to_string()
                .into();
            TreeItem::new(relative, label)
        }))
        .collect()
}

const SKIP_DIRS: &[&str] = &["target", "node_modules", "dist", "build"];

fn is_source_file(name: &str) -> bool {
    name.ends_with(".rs") || name.ends_with(".rml") || name.ends_with(".toml")
}

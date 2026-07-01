//! 资源加载运行时
//!
//! 由 `build.rs` 的 `AssetsProcessor` 在编译期生成注册代码到
//! `OUT_DIR/rml_generated/rml_assets.rs`,内含一个 `#[ctor::ctor]` 函数,
//! 在 `main` 之前自动调用 `assets::init(...)` 完成注册。
//!
//! 支持两种模式（由 `build.rs` 的 `.assets(path, embed)` 决定）：
//! - **Embedded**：所有文件经 `include_bytes!` 编译期嵌入二进制
//! - **Filesystem**：运行期从 `{root}/{path}` 读取,首次读取后用 `Box::leak` 缓存
//!   到 `'static`（用户已声明不关心资源泄露）
//!
//! 运行时通过 `load(path)` / `load_str(path)` 查询,路径以相对 `assets/` 的
//! 正斜杠形式（如 `"themes/dark.css"`、`"i18n/zh-CN.json"`）。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 资源来源模式
pub enum AssetSource {
    /// 嵌入模式：编译期通过 `include_bytes!` 嵌入二进制
    Embedded {
        entries: &'static [(&'static str, &'static [u8])],
    },
    /// 文件系统模式：运行期从磁盘读取,首次读取后缓存
    Filesystem {
        /// 编译期固化的绝对路径根（CARGO_MANIFEST_DIR + assets_dir）
        root: &'static str,
    },
}

/// 全局资源源（由 `init` 设置；通常由 build.rs 生成的 `#[ctor]` 函数自动调用）
static ASSETS: OnceLock<AssetSource> = OnceLock::new();

/// 文件系统模式下的字节缓存（路径 → `'static [u8]`）
///
/// 仅 `Filesystem` 模式使用。`Box::leak` 让磁盘读出的字节获得 `'static` 生命周期,
/// 以满足 `load() -> Option<&'static [u8]>` 签名。代价是这些字节不释放,
/// 符合方案 2「不关心资源泄露」的前提。
static FS_CACHE: Mutex<Option<HashMap<String, &'static [u8]>>> = Mutex::new(None);

/// 注册资源源
///
/// 通常由 `build.rs` 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动调用,
/// 无需用户手动调用。
pub fn init(source: AssetSource) {
    let _ = ASSETS.set(source);
}

/// 资源源是否已注册
pub fn is_initialized() -> bool {
    ASSETS.get().is_some()
}

/// 按路径加载资源字节
///
/// `path` 为相对 `assets/` 的正斜杠路径（如 `"themes/dark.css"`）。
/// - 嵌入模式：直接返回编译期嵌入的 `&'static [u8]`
/// - 文件系统模式：首次从磁盘读取并缓存,后续命中缓存
///
/// 未注册或路径不存在时返回 `None`。
pub fn load(path: &str) -> Option<&'static [u8]> {
    let normalized = normalize(path);
    match ASSETS.get()? {
        AssetSource::Embedded { entries } => entries
            .iter()
            .find(|(p, _)| *p == normalized)
            .map(|(_, b)| *b),
        AssetSource::Filesystem { root } => load_from_fs(root, &normalized),
    }
}

/// 按路径加载资源文本
///
/// 内部调用 `load` 并尝试 UTF-8 解码。非 UTF-8 资源返回 `None`。
pub fn load_str(path: &str) -> Option<&'static str> {
    load(path).and_then(|b| std::str::from_utf8(b).ok())
}

/// 列出所有嵌入资源的路径（主要用于调试；文件系统模式返回空）
pub fn list() -> Vec<&'static str> {
    match ASSETS.get() {
        Some(AssetSource::Embedded { entries }) => {
            entries.iter().map(|(p, _)| *p).collect()
        }
        _ => Vec::new(),
    }
}

/// 文件系统模式：从 `{root}/{path}` 读取字节,缓存后返回 `&'static [u8]`
fn load_from_fs(root: &str, normalized: &str) -> Option<&'static [u8]> {
    // 先查缓存
    {
        let guard = FS_CACHE.lock().ok()?;
        if let Some(cache) = guard.as_ref() {
            if let Some(&bytes) = cache.get(normalized) {
                return Some(bytes);
            }
        }
    }
    // 缓存未命中：从磁盘读取
    let full_path = format!("{}/{}", root.trim_end_matches('/'), normalized);
    let bytes = std::fs::read(&full_path).ok()?;
    // Box::leak 让字节获得 'static 生命周期（方案 2 已声明不关心资源泄露）
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    // 写入缓存
    if let Ok(mut guard) = FS_CACHE.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(normalized.to_string(), leaked);
    }
    Some(leaked)
}

/// 路径归一化：反斜杠转正斜杠、去掉前导 `/`
fn normalize(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    replaced.trim_start_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    static TEST_ASSETS: &[(&str, &[u8])] = &[
        ("themes/dark.css", b":root { --primary: #000; }"),
        ("i18n/zh-CN.json", b"{\"hello\": \"world\"}"),
    ];

    fn init_test_embedded() {
        // 用一个新 OnceLock 不行,因为 ASSETS 是全局静态的。
        // 测试中先确保未初始化或已初始化为 Embedded 模式。
        if !is_initialized() {
            init(AssetSource::Embedded { entries: TEST_ASSETS });
        }
    }

    #[test]
    fn normalize_strips_leading_slash() {
        assert_eq!(normalize("/themes/dark.css"), "themes/dark.css");
        assert_eq!(normalize("themes/dark.css"), "themes/dark.css");
        assert_eq!(normalize("\\themes\\dark.css"), "themes/dark.css");
    }

    #[test]
    fn load_returns_bytes_embedded() {
        init_test_embedded();
        let bytes = load("themes/dark.css");
        assert!(bytes.is_some());
        assert_eq!(bytes.unwrap(), b":root { --primary: #000; }");
    }

    #[test]
    fn load_str_returns_text_embedded() {
        init_test_embedded();
        let text = load_str("i18n/zh-CN.json");
        assert!(text.is_some());
        assert!(text.unwrap().contains("hello"));
    }

    #[test]
    fn load_missing_returns_none() {
        init_test_embedded();
        assert!(load("nonexistent.txt").is_none());
    }

    #[test]
    fn list_returns_all_paths_embedded() {
        init_test_embedded();
        let paths = list();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"themes/dark.css"));
        assert!(paths.contains(&"i18n/zh-CN.json"));
    }

    #[test]
    fn load_from_fs_caches_bytes() {
        // 准备临时目录
        let tmp: PathBuf = std::env::temp_dir().join("rml_assets_fs_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("themes")).unwrap();
        {
            let mut f = std::fs::File::create(tmp.join("themes/dark.css")).unwrap();
            writeln!(f, ":root {{ --primary: #000; }}").unwrap();
        }

        // 此测试仅在 ASSETS 未被前序测试设置时有效,否则会被跳过
        if is_initialized() {
            // ASSETS 已被前序测试设置为 Embedded,本测试无法切换。
            // 由 fs_test 独立 binary 验证 Filesystem 模式。
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }

        init(AssetSource::Filesystem {
            root: Box::leak(tmp.to_string_lossy().into_owned().into_boxed_str()),
        });
        let bytes = load("themes/dark.css");
        assert!(bytes.is_some());
        assert!(std::str::from_utf8(bytes.unwrap()).unwrap().contains("--primary"));
        // 再次读取应命中缓存
        let bytes2 = load("themes/dark.css");
        assert!(bytes2.is_some());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

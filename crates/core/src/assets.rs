//! 资源嵌入运行时查询
//!
//! 资源注册表由 `build.rs` 的 `AssetsProcessor` 生成到
//! `OUT_DIR/rml_generated/rml_assets.rs`,通过 `rml::main!()` 宏一键完成
//! 注入与注册(内部调用 `embed_assets!()` + `assets::init()`)。
//!
//! 运行时通过 `load(path)` / `load_str(path)` 查询,路径以相对 `assets/` 的
//! 正斜杠形式(如 `"themes/dark.css"`、`"i18n/zh-CN.json"`)。

use std::sync::OnceLock;

/// 全局资源注册表(由 `init` 设置)
static ASSETS: OnceLock<&'static [(&'static str, &'static [u8])]> = OnceLock::new();

/// 注册嵌入资源表
///
/// 通常由 `rml::main!()` 宏内部调用,无需手动调用。
/// 如需自定义启动流程,可手动调用:
/// ```rust,ignore
/// rml::embed_assets!();
/// fn main() {
///     rml::assets::init(RML_ASSETS);
///     // ...
/// }
/// ```
pub fn init(assets: &'static [(&'static str, &'static [u8])]) {
    let _ = ASSETS.set(assets);
}

/// 资源是否已注册
pub fn is_initialized() -> bool {
    ASSETS.get().is_some()
}

/// 按路径加载资源字节
///
/// `path` 为相对 `assets/` 的正斜杠路径(如 `"themes/dark.css"`)。
/// 未注册或路径不存在时返回 `None`。
pub fn load(path: &str) -> Option<&'static [u8]> {
    let normalized = normalize(path);
    ASSETS
        .get()?
        .iter()
        .find(|(p, _)| *p == normalized)
        .map(|(_, b)| *b)
}

/// 按路径加载资源文本
///
/// 内部调用 `load` 并尝试 UTF-8 解码。非 UTF-8 资源返回 `None`。
pub fn load_str(path: &str) -> Option<&'static str> {
    load(path).and_then(|b| std::str::from_utf8(b).ok())
}

/// 列出所有嵌入资源的路径(主要用于调试)
pub fn list() -> Vec<&'static str> {
    ASSETS
        .get()
        .map(|a| a.iter().map(|(p, _)| *p).collect())
        .unwrap_or_default()
}

/// 路径归一化:反斜杠转正斜杠、去掉前导 `/`
fn normalize(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    replaced.trim_start_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_ASSETS: &[(&str, &[u8])] = &[
        ("themes/dark.css", b":root { --primary: #000; }"),
        ("i18n/zh-CN.json", b"{\"hello\": \"world\"}"),
    ];

    #[test]
    fn normalize_strips_leading_slash() {
        assert_eq!(normalize("/themes/dark.css"), "themes/dark.css");
        assert_eq!(normalize("themes/dark.css"), "themes/dark.css");
        assert_eq!(normalize("\\themes\\dark.css"), "themes/dark.css");
    }

    #[test]
    fn load_returns_bytes() {
        init(TEST_ASSETS);
        let bytes = load("themes/dark.css");
        assert!(bytes.is_some());
        assert_eq!(bytes.unwrap(), b":root { --primary: #000; }");
    }

    #[test]
    fn load_str_returns_text() {
        init(TEST_ASSETS);
        let text = load_str("i18n/zh-CN.json");
        assert!(text.is_some());
        assert!(text.unwrap().contains("hello"));
    }

    #[test]
    fn load_missing_returns_none() {
        init(TEST_ASSETS);
        assert!(load("nonexistent.txt").is_none());
    }

    #[test]
    fn list_returns_all_paths() {
        init(TEST_ASSETS);
        let paths = list();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"themes/dark.css"));
        assert!(paths.contains(&"i18n/zh-CN.json"));
    }
}

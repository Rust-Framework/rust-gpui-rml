//! Workspace 资源索引:i18n JSON 与 CSS 类声明
//!
//! 在 LSP `initialized` 时扫描 `root_path` 下 `**/i18n/*.json` 与 `**/*.css`,
//! 构建 key → locale 翻译、class → 声明列表的反查表,供 hover 查询使用。
//!
//! JSON 扁平化逻辑本地实现(lsp crate 不依赖 rust-rml-core,避免拉入 gpui)。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::Url;
use rust_rml_engine::css::ast::{Declaration, Selector, Value};
use rust_rml_engine::css::parser;

// ──────────────────────────────────────────────────────────────────────────
// i18n 索引
// ──────────────────────────────────────────────────────────────────────────

/// i18n 翻译条目(单个 locale 的单条翻译)
#[derive(Debug, Clone)]
pub struct I18nEntry {
    /// locale 名(文件名 stem,如 "zh-CN")
    pub locale: String,
    /// 翻译文本
    pub value: String,
    /// 来源文件 URI
    pub file_uri: Url,
}

/// i18n 资源索引:key → 各 locale 翻译列表
#[derive(Debug, Default)]
pub struct I18nIndex {
    entries: HashMap<String, Vec<I18nEntry>>,
}

impl I18nIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// 扫描 `root_path` 下所有 `**/i18n/*.json` 文件
    pub fn scan(&mut self, root_path: &Path) {
        let mut json_files = Vec::new();
        collect_i18n_json(root_path, &mut json_files);
        for file in json_files {
            self.load_file(&file);
        }
    }

    fn load_file(&mut self, path: &Path) {
        let Ok(text) = std::fs::read_to_string(path) else {
            log::warn!("failed to read i18n file: {:?}", path);
            return;
        };
        let Ok(value): std::result::Result<serde_json::Value, _> =
            serde_json::from_str(&text)
        else {
            log::warn!("invalid i18n JSON: {:?}", path);
            return;
        };
        let locale = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("default")
            .to_string();
        let Ok(uri) = Url::from_file_path(path) else {
            return;
        };
        let mut flat: HashMap<String, String> = HashMap::new();
        flatten_json_value(&value, "", &mut flat);
        for (key, val) in flat {
            self.entries.entry(key).or_default().push(I18nEntry {
                locale: locale.clone(),
                value: val,
                file_uri: uri.clone(),
            });
        }
    }

    /// 查询 key 的所有 locale 翻译
    pub fn lookup(&self, key: &str) -> Option<&Vec<I18nEntry>> {
        self.entries.get(key)
    }

    /// 已索引的 key 数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 递归扁平化嵌套 JSON 对象为点路径 key
///
/// 复制自 `crates/core/src/i18n.rs::flatten_json_value` 逻辑,
/// 因 lsp crate 不依赖 rust-rml-core(避免拉入 gpui),故本地实现。
fn flatten_json_value(
    value: &serde_json::Value,
    prefix: &str,
    out: &mut HashMap<String, String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_json_value(v, &key, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        serde_json::Value::Number(n) => {
            out.insert(prefix.to_string(), n.to_string());
        }
        serde_json::Value::Bool(b) => {
            out.insert(prefix.to_string(), b.to_string());
        }
        _ => {}
    }
}

/// 递归收集 `i18n` 子目录下的 `.json` 文件
fn collect_i18n_json(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // 命中 i18n 目录时,扫描其下所有 .json
            if path.file_name().map_or(false, |n| n == "i18n") {
                collect_json_in_dir(&path, out);
            } else {
                collect_i18n_json(&path, out);
            }
        }
    }
}

/// 收取单个目录下的所有 `.json` 文件(不递归)
fn collect_json_in_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "json") {
            out.push(path);
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// CSS 索引
// ──────────────────────────────────────────────────────────────────────────

/// CSS class 声明条目(单个文件中的单个规则块)
#[derive(Debug, Clone)]
pub struct CssClassEntry {
    /// 声明列表 `(property, value 文本)`
    pub declarations: Vec<(String, String)>,
    /// 来源文件 URI
    pub file_uri: Url,
}

/// CSS class 索引:class 名 → 各文件中的声明
#[derive(Debug, Default)]
pub struct CssIndex {
    entries: HashMap<String, Vec<CssClassEntry>>,
}

impl CssIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// 扫描 `root_path` 下所有 `**/*.css` 文件
    pub fn scan(&mut self, root_path: &Path) {
        let mut css_files = Vec::new();
        collect_css_files(root_path, &mut css_files);
        for file in css_files {
            self.load_file(&file);
        }
    }

    fn load_file(&mut self, path: &Path) {
        let Ok(text) = std::fs::read_to_string(path) else {
            log::warn!("failed to read css file: {:?}", path);
            return;
        };
        let Ok(sheet) = parser::parse(&text) else {
            log::warn!("css parse failed: {:?}", path);
            return;
        };
        let Ok(uri) = Url::from_file_path(path) else {
            return;
        };
        for rule in &sheet.rules {
            let decls = render_declarations(&rule.declarations);
            if decls.is_empty() {
                continue;
            }
            // 从每个 selector 中提取所有 class 名(含复合/后代/子选择器)
            let mut class_names = Vec::new();
            for sel in &rule.selectors {
                collect_class_names(sel, &mut class_names);
            }
            for name in class_names {
                self.entries
                    .entry(name)
                    .or_default()
                    .push(CssClassEntry {
                        declarations: decls.clone(),
                        file_uri: uri.clone(),
                    });
            }
        }
    }

    /// 查询 class 名的所有声明
    pub fn lookup(&self, class_name: &str) -> Option<&Vec<CssClassEntry>> {
        self.entries.get(class_name)
    }

    /// 已索引的 class 数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 递归收集 CSS 目录下所有 `.css` 文件
fn collect_css_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // 跳过 target/build 目录,避免扫描构建产物
            if path
                .file_name()
                .map_or(false, |n| n == "target" || n == "node_modules")
            {
                continue;
            }
            collect_css_files(&path, out);
        } else if path.extension().map_or(false, |e| e == "css") {
            out.push(path);
        }
    }
}

/// 从选择器中递归提取所有 class 名
///
/// 支持复合选择器(`.button.primary`)、后代选择器(`.container .item`)、
/// 子选择器(`.list > .item`)等。
fn collect_class_names(sel: &Selector, out: &mut Vec<String>) {
    match sel {
        Selector::Class(name) => out.push(name.clone()),
        Selector::Compound(parts) => {
            for p in parts {
                collect_class_names(p, out);
            }
        }
        Selector::Descendant(parent, child) => {
            collect_class_names(parent, out);
            collect_class_names(child, out);
        }
        Selector::Child(parent, child) => {
            collect_class_names(parent, out);
            collect_class_names(child, out);
        }
        _ => {}
    }
}

/// 将声明列表渲染为 `(property, value_text)` 元组
///
/// `value_text` 为 CSS 值的可读文本形式(如 `"10px"`、`"#fff"`、`"flex"`),
/// 供 hover 显示。
fn render_declarations(decls: &[Declaration]) -> Vec<(String, String)> {
    decls
        .iter()
        .map(|d| (d.property.clone(), render_value(&d.value)))
        .collect()
}

/// 将 CSS `Value` 渲染为可读文本
fn render_value(v: &Value) -> String {
    match v {
        Value::Length(n, unit) => {
            let unit_str = match unit {
                rust_rml_engine::css::ast::Unit::Px => "px",
                rust_rml_engine::css::ast::Unit::Pt => "pt",
                rust_rml_engine::css::ast::Unit::Em => "em",
                rust_rml_engine::css::ast::Unit::Rem => "rem",
                rust_rml_engine::css::ast::Unit::Percent => "%",
                rust_rml_engine::css::ast::Unit::Vw => "vw",
                rust_rml_engine::css::ast::Unit::Vh => "vh",
            };
            format!("{}{}", n, unit_str)
        }
        Value::Color(c) => format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
        Value::Number(n) => n.to_string(),
        Value::Keyword(s) => s.clone(),
        Value::String(s) => s.clone(),
        Value::Var(name, fallback) => match fallback {
            Some(fb) => format!("var({}, {})", name, render_value(fb)),
            None => format!("var({})", name),
        },
        Value::Function(name, args) => {
            let args_str: Vec<String> = args.iter().map(render_value).collect();
            format!("{}({})", name, args_str.join(", "))
        }
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(render_value).collect();
            parts.join(" ")
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_nested_json_object() {
        let json = r#"{"a": {"b": {"c": "value"}}, "flat": "f"}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut out = HashMap::new();
        flatten_json_value(&value, "", &mut out);
        assert_eq!(out.get("a.b.c"), Some(&"value".to_string()));
        assert_eq!(out.get("flat"), Some(&"f".to_string()));
    }

    #[test]
    fn flatten_json_handles_non_string_values() {
        let json = r#"{"n": 42, "b": true}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut out = HashMap::new();
        flatten_json_value(&value, "", &mut out);
        assert_eq!(out.get("n"), Some(&"42".to_string()));
        assert_eq!(out.get("b"), Some(&"true".to_string()));
    }

    #[test]
    fn collect_class_names_from_simple_selector() {
        let mut out = Vec::new();
        collect_class_names(&Selector::Class("foo".into()), &mut out);
        assert_eq!(out, vec!["foo".to_string()]);
    }

    #[test]
    fn collect_class_names_from_compound_selector() {
        let compound = Selector::Compound(vec![
            Selector::Class("button".into()),
            Selector::Class("primary".into()),
        ]);
        let mut out = Vec::new();
        collect_class_names(&compound, &mut out);
        assert_eq!(out, vec!["button".to_string(), "primary".to_string()]);
    }

    #[test]
    fn collect_class_names_from_descendant_selector() {
        let desc = Selector::Descendant(
            Box::new(Selector::Class("container".into())),
            Box::new(Selector::Class("item".into())),
        );
        let mut out = Vec::new();
        collect_class_names(&desc, &mut out);
        assert_eq!(out, vec!["container".to_string(), "item".to_string()]);
    }

    #[test]
    fn render_length_value() {
        assert_eq!(render_value(&Value::Length(10.0, rust_rml_engine::css::ast::Unit::Px)), "10px");
        assert_eq!(render_value(&Value::Length(50.0, rust_rml_engine::css::ast::Unit::Percent)), "50%");
    }

    #[test]
    fn render_color_value() {
        let c = rust_rml_engine::css::Color::rgb(255, 0, 128);
        assert_eq!(render_value(&Value::Color(c)), "#ff0080");
    }

    #[test]
    fn css_index_scan_extracts_class_rules() {
        let tmp = std::env::temp_dir().join("rml_lsp_assets_test_css_simple");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let css_path = tmp.join("test.css");
        std::fs::write(
            &css_path,
            ".case-pane {\n  display: flex;\n  padding: 24px;\n}\n.login {\n  height: 100%;\n}\n",
        )
        .unwrap();

        let mut idx = CssIndex::new();
        idx.scan(&tmp);
        assert_eq!(idx.lookup("case-pane").map(|v| v.len()), Some(1));
        assert_eq!(idx.lookup("login").map(|v| v.len()), Some(1));
        let entries = idx.lookup("case-pane").unwrap();
        assert!(entries[0]
            .declarations
            .iter()
            .any(|(p, v)| p == "display" && v == "flex"));
        assert!(entries[0]
            .declarations
            .iter()
            .any(|(p, v)| p == "padding" && v == "24px"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn css_index_scan_extracts_compound_selector() {
        let tmp = std::env::temp_dir().join("rml_lsp_assets_test_css_compound");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let css_path = tmp.join("compound.css");
        std::fs::write(&css_path, ".button.primary {\n  color: #fff;\n}\n").unwrap();

        let mut idx = CssIndex::new();
        idx.scan(&tmp);
        assert_eq!(idx.lookup("button").map(|v| v.len()), Some(1));
        assert_eq!(idx.lookup("primary").map(|v| v.len()), Some(1));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn i18n_index_scan_loads_json() {
        let tmp = std::env::temp_dir().join("rml_lsp_assets_test_i18n");
        let _ = std::fs::remove_dir_all(&tmp);
        let i18n_dir = tmp.join("i18n");
        std::fs::create_dir_all(&i18n_dir).unwrap();
        std::fs::write(
            i18n_dir.join("zh-CN.json"),
            r#"{"login.title": "登录", "login.hint": "提示"}"#,
        )
        .unwrap();
        std::fs::write(
            i18n_dir.join("en-US.json"),
            r#"{"login.title": "Login", "login.hint": "Hint"}"#,
        )
        .unwrap();

        let mut idx = I18nIndex::new();
        idx.scan(&tmp);
        let entries = idx.lookup("login.title").unwrap();
        assert_eq!(entries.len(), 2);
        let locales: Vec<&str> = entries.iter().map(|e| e.locale.as_str()).collect();
        assert!(locales.contains(&"zh-CN"));
        assert!(locales.contains(&"en-US"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn i18n_index_scan_handles_nested_json() {
        let tmp = std::env::temp_dir().join("rml_lsp_assets_test_i18n_nested");
        let _ = std::fs::remove_dir_all(&tmp);
        let i18n_dir = tmp.join("i18n");
        std::fs::create_dir_all(&i18n_dir).unwrap();
        std::fs::write(
            i18n_dir.join("zh-CN.json"),
            r#"{"login": {"title": "登录", "hint": "提示"}}"#,
        )
        .unwrap();

        let mut idx = I18nIndex::new();
        idx.scan(&tmp);
        assert!(idx.lookup("login.title").is_some());
        assert_eq!(
            idx.lookup("login.title").unwrap()[0].value,
            "登录"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

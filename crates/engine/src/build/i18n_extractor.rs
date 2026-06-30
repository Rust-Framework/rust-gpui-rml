//! 从 `.rml` 源文件扫描 `{t("key")}` 调用，合并写入 i18n JSON 资源

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 构建期 i18n 键提取器
///
/// 扫描 RML 模板中的 `t("...")` / `t('...')` 调用，将缺失的 key 以 key 本身为默认值
/// 写入目标 JSON 文件（不覆盖已有翻译）。
pub struct I18nExtractor {
    path: PathBuf,
}

impl I18nExtractor {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 从多个 RML 源文本中提取并合并 key
    pub fn extract_from_sources(&self, sources: &[&str]) -> Result<(), String> {
        let mut keys = BTreeMap::new();
        for source in sources {
            collect_t_keys(source, &mut keys);
        }
        if keys.is_empty() {
            return Ok(());
        }
        merge_keys_into_json(&self.path, &keys)
    }
}

/// 扫描 `t("key")` / `t('key')` 字面量调用
fn collect_t_keys(source: &str, out: &mut BTreeMap<String, ()>) {
    let bytes = source.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b't' && bytes[i + 1] == b'(' {
            if let Some((key, end)) = parse_string_arg(&source[i + 2..]) {
                out.insert(key, ());
                i += 2 + end;
                continue;
            }
        }
        i += 1;
    }
}

/// 解析 `("key")` 或 `('key')` 前缀，返回 key 与消耗长度（含闭括号）
fn parse_string_arg(rest: &str) -> Option<(String, usize)> {
    let rest = rest.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut key = String::new();
    let mut escaped = false;
    let mut consumed = 1; // opening quote
    for ch in rest[1..].chars() {
        consumed += ch.len_utf8();
        if escaped {
            key.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            c if c == quote => {
                let after = rest[consumed..].trim_start();
                if after.starts_with(')') {
                    consumed += rest[consumed..].len() - after.len() + 1;
                    return Some((key, consumed));
                }
                return None;
            }
            c => key.push(c),
        }
    }
    None
}

fn merge_keys_into_json(path: &Path, keys: &BTreeMap<String, ()>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }

    let mut map: BTreeMap<String, serde_json::Value> = if path.exists() {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|e| format!("parse {}: {e}", path.display()))?
    } else {
        BTreeMap::new()
    };

    for key in keys.keys() {
        map.entry(key.clone())
            .or_insert_with(|| serde_json::Value::String(key.clone()));
    }

    let value = serde_json::Value::Object(map.into_iter().collect());
    let text = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("serialize i18n: {e}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_t_calls() {
        let src = r#"<div>{t("app.title")}</div>  {t('menu.file')}"#;
        let mut keys = BTreeMap::new();
        collect_t_keys(src, &mut keys);
        assert!(keys.contains_key("app.title"));
        assert!(keys.contains_key("menu.file"));
    }
}

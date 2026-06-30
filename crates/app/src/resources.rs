//! 资源加载
//!
//! i18n JSON 资源从 `assets/i18n/{locale}.json` 加载。

use std::collections::HashMap;

pub use rml_core::i18n::{catalog_from_json, load_catalog_from_dir, DEFAULT_I18N_DIR};

/// 加载指定 locale 的 i18n catalog
pub fn load_i18n_catalog(
    locale: &str,
    dir: &str,
) -> Result<HashMap<String, String>, String> {
    load_catalog_from_dir(locale, dir)
}

/// 从 JSON 字符串加载 catalog
pub fn load_i18n_from_json(json: &str) -> Result<HashMap<String, String>, String> {
    catalog_from_json(json)
}

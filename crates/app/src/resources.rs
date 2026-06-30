//! 资源加载
//!
//! i18n JSON 资源从 `assets/i18n/{locale}.json` 加载。
//! 主题 CSS 资源从 `assets/themes/{theme}.css` 加载。

use std::collections::HashMap;

use gpui::Rgba;

pub use rml_core::i18n::{
    catalog_from_json, load_catalog_embedded, load_catalog_from_dir, DEFAULT_I18N_DIR,
};
pub use rml_core::theme::{parse_theme_css, DEFAULT_THEMES_DIR};

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

/// 从嵌入资源加载主题 CSS 文本
///
/// `themes_dir` 为相对 `assets/` 根的主题目录(如 `"assets/themes"`),
/// 资源路径为 `{themes_dir}/{theme}.css`。
pub fn load_theme_css(theme: &str, themes_dir: &str) -> Result<String, String> {
    let path = format!("{}/{}.css", themes_dir.trim_end_matches('/'), theme);
    rml_core::assets::load_str(&path)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("theme asset not embedded: {}", path))
}

/// 从嵌入资源加载并解析主题颜色表
pub fn load_theme_colors(theme: &str, themes_dir: &str) -> Result<HashMap<String, Rgba>, String> {
    let css = load_theme_css(theme, themes_dir)?;
    parse_theme_css(&css)
}


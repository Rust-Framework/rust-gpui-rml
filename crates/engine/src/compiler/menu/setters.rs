//! menu / MenuBar / status_bar 专用 bind setter。
//!
//! 由 `component::component_bind_setter` 在 tag 匹配 menu 类标签时委托调用。

use crate::tags;

/// menu / MenuBar / status_bar 专用 bind setter
///
/// `items={expr}` → `.items(self.<expr>.clone())`
///
/// 由 `component::component_bind_setter` 在 tag 匹配 menu 类标签时委托调用。
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    match name {
        "items"
            if matches!(
                tags::normalize_component_tag(tag).as_str(),
                "menu" | "MenuBar" | "status_bar"
            ) =>
        {
            let rust_expr =
                super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".items({}.clone())", rust_expr))
        }
        _ => None,
    }
}

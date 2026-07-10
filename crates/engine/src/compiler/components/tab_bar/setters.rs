//! 原生 TabBar 专用属性 → builder 方法映射。
//!
//! 与 `tabs::setters` 的关键差异：
//! - **不含 `bordered`**：TabBar 是纯 header，无边框概念
//! - **不含 `on_close`/`on_close_all`/`on_close_others`/`on_promote`**：TabBar 不暴露关闭/提升
//!
//! `<tab>` 子节点属性由 `tabs::tab::gen_tab_child` 处理（调用 `tabs::setters` with tag "Tab"），
//! 本模块仅处理 TabBar 容器属性（tag "TabBar"）。

use crate::parser::ast::EventHandler;

/// 静态属性 → builder 方法（仅 TabBar 容器）
///
/// - `variant="underline"` / `"pill"` / `"flat"` / `"outline"` / `"segmented"` → `.<variant>()`
/// - `variant="tab"` = 默认，no-op
/// - `menu="true"` → `.menu(true)`
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    match name {
        // variant="underline" → .underline() 等
        // variant="tab" = 默认 TabVariant::Tab，no-op
        "variant" if tag == "TabBar" => match value {
            "flat" | "outline" | "pill" | "segmented" | "underline" => Some(format!(".{}()", value)),
            "tab" => Some(String::new()),
            _ => None,
        },
        "menu" if tag == "TabBar" => {
            let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".menu({})", bool_val))
        }
        _ => None,
    }
}

/// 绑定属性 → builder 方法（仅 TabBar 容器）
///
/// - `selected_index={expr}` → `.selected_index(<expr>)`
/// - `prefix={expr}` / `suffix={expr}` → `.<name>(<expr>)`
/// - `last_empty_space={expr}` → `.last_empty_space(<expr>)`
/// - `track_scroll={expr}` → `.track_scroll(&<expr>)`（ScrollHandle 引用）
/// - `menu={bool expr}` → `.menu(<expr>)`
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    match name {
        "selected_index" | "menu" | "last_empty_space" if tag == "TabBar" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".{}({})", name, rust_expr))
        }
        "track_scroll" if tag == "TabBar" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".track_scroll(&{})", rust_expr))
        }
        "prefix" | "suffix" if tag == "TabBar" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".{}({})", name, rust_expr))
        }
        _ => None,
    }
}

/// 事件属性 → builder 方法（仅 TabBar 容器）
///
/// ## TabBar 的 on_click
/// `.on_click(cx.listener(move |this, idx: &usize, _window, cx| { this.<method>(*idx, cx); }))`
/// 用户方法签名约定：`fn method(&mut self, index: usize, cx: &mut Context<Self>)`
///
/// TabBar 不支持 on_close/on_close_all/on_close_others/on_promote（返回 None）。
pub fn event_setter(name: &str, handler: &EventHandler, tag: &str) -> Option<String> {
    if tag != "TabBar" {
        return None;
    }
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
    };
    match name {
        "on_click" => Some(format!(
            ".on_click(cx.listener(move |this, idx: &usize, _window, cx| {{\n                    \
             this.{method}(*idx, cx);\n                }}))",
            method = method
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_tab_bar_variants() {
        assert_eq!(static_setter("variant", "underline", "TabBar").unwrap(), ".underline()");
        assert_eq!(static_setter("variant", "pill", "TabBar").unwrap(), ".pill()");
        assert_eq!(static_setter("variant", "flat", "TabBar").unwrap(), ".flat()");
        assert_eq!(static_setter("variant", "outline", "TabBar").unwrap(), ".outline()");
        assert_eq!(static_setter("variant", "segmented", "TabBar").unwrap(), ".segmented()");
    }

    #[test]
    fn static_setter_tab_bar_variant_tab_default_no_op() {
        // variant="tab" = 默认 TabVariant::Tab，返回空字符串 no-op
        assert_eq!(static_setter("variant", "tab", "TabBar").unwrap(), "");
    }

    #[test]
    fn static_setter_variant_invalid_returns_none() {
        assert!(static_setter("variant", "invalid", "TabBar").is_none());
        assert!(static_setter("variant", "false", "TabBar").is_none());
    }

    #[test]
    fn static_setter_tab_bar_menu() {
        assert_eq!(static_setter("menu", "", "TabBar").unwrap(), ".menu(true)");
        assert_eq!(static_setter("menu", "true", "TabBar").unwrap(), ".menu(true)");
        assert_eq!(static_setter("menu", "false", "TabBar").unwrap(), ".menu(false)");
    }

    #[test]
    fn static_setter_bordered_not_supported() {
        // TabBar 不支持 bordered（纯 header，无边框概念）
        assert!(static_setter("bordered", "true", "TabBar").is_none());
    }

    #[test]
    fn static_setter_unknown_returns_none() {
        assert!(static_setter("label", "x", "TabBar").is_none());
        assert!(static_setter("selected_index", "0", "TabBar").is_none());
        assert!(static_setter("foo", "bar", "TabBar").is_none());
    }

    #[test]
    fn bind_setter_tab_bar_selected_index() {
        let code = bind_setter("selected_index", "active", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".selected_index(self.active)");
    }

    #[test]
    fn bind_setter_tab_bar_menu() {
        let code = bind_setter("menu", "show_menu", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".menu(self.show_menu)");
    }

    #[test]
    fn bind_setter_tab_bar_last_empty_space() {
        let code = bind_setter("last_empty_space", "spacer", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".last_empty_space(self.spacer)");
    }

    #[test]
    fn bind_setter_tab_bar_track_scroll() {
        let code = bind_setter("track_scroll", "my_scroll", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".track_scroll(&self.my_scroll)");
    }

    #[test]
    fn bind_setter_tab_bar_track_scroll_with_loop_var() {
        let code = bind_setter("track_scroll", "item.scroll", &["item"], &[], "TabBar").unwrap();
        assert_eq!(code, ".track_scroll(&item.scroll)");
    }

    #[test]
    fn bind_setter_prefix_suffix() {
        let code = bind_setter("prefix", "back_btn", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".prefix(self.back_btn)");
        let code = bind_setter("suffix", "more_btn", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".suffix(self.more_btn)");
    }

    #[test]
    fn bind_setter_bordered_not_supported() {
        // TabBar 不支持 bordered
        assert!(bind_setter("bordered", "has_border", &[], &[], "TabBar").is_none());
    }

    #[test]
    fn bind_setter_unknown_returns_none() {
        assert!(bind_setter("value", "x", &[], &[], "TabBar").is_none());
    }

    #[test]
    fn event_setter_tab_bar_on_click() {
        let handler = EventHandler::Ident("on_tab_select".into());
        let code = event_setter("on_click", &handler, "TabBar").unwrap();
        assert!(code.starts_with(".on_click("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("idx: &usize"));
        assert!(code.contains("this.on_tab_select(*idx, cx)"));
    }

    #[test]
    fn event_setter_on_close_not_supported() {
        // TabBar 不支持 on_close
        let handler = EventHandler::Ident("on_tab_close".into());
        assert!(event_setter("on_close", &handler, "TabBar").is_none());
    }

    #[test]
    fn event_setter_on_close_all_not_supported() {
        // TabBar 不支持 on_close_all
        let handler = EventHandler::Ident("on_tab_close_all".into());
        assert!(event_setter("on_close_all", &handler, "TabBar").is_none());
    }

    #[test]
    fn event_setter_on_promote_not_supported() {
        // TabBar 不支持 on_promote
        let handler = EventHandler::Ident("on_tab_promote".into());
        assert!(event_setter("on_promote", &handler, "TabBar").is_none());
    }

    #[test]
    fn event_setter_unknown_event_returns_none() {
        let handler = EventHandler::Ident("h".into());
        assert!(event_setter("on_change", &handler, "TabBar").is_none());
    }
}

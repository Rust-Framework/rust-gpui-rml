//! TabBar / Tab 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter` /
//! `component_event_setter` 在 tag 为 "TabBar" 或 "Tab" 时委托调用。
//! 未命中返回 None，由公共 setter 回退到通用属性（Sizable、disabled 等）。
//!
//! ## on_click 签名差异
//!
//! - TabBar：`Fn(&usize, &mut Window, &mut App)` —— 传入被点击的 tab 索引
//! - Tab：`Fn(&ClickEvent, &mut Window, &mut App)` —— 标准 GPUI 点击事件
//!
//! 在 `event_setter` 中按 tag 参数区分。

use crate::parser::ast::EventHandler;

/// 静态属性 → builder 方法
///
/// - `underline=""` / `pill=""` / `flat=""` / `outline=""` / `segmented=""` → `.<name>()`（variant 快捷方法）
/// - `menu="true"` → `.menu(true)`（仅 TabBar）
/// - `icon="User"` → `.icon(rml_ui::IconName::User)`（仅 Tab）
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    match name {
        "underline" | "pill" | "flat" | "outline" | "segmented" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".{}()", name))
            } else {
                None
            }
        }
        "menu" if tag == "TabBar" => {
            let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".menu({})", bool_val))
        }
        "icon" if tag == "Tab" => Some(format!(".icon(rml_ui::IconName::{})", value)),
        "title" if tag == "TabItem" => Some(format!(".title({:?})", value)),
        "title_icon" if tag == "TabItem" => Some(format!(".title_icon(rml_ui::IconName::{})", value)),
        _ => None,
    }
}

/// 绑定属性 → builder 方法
///
/// - `selected_index={expr}` → `.selected_index(<expr>)`（TabBar）
/// - `prefix={expr}` / `suffix={expr}` → `.<name>(<expr>)`（element，不加 .clone()）
/// - `last_empty_space={expr}` → `.last_empty_space(<expr>)`（TabBar）
/// - `menu={bool expr}` → `.menu(<expr>)`（TabBar）
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    match name {
        "selected_index" | "menu" | "last_empty_space" if tag == "TabBar" => {
            let rust_expr = super::super::component::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".{}({})", name, rust_expr))
        }
        "prefix" | "suffix" => {
            let rust_expr = super::super::component::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".{}({})", name, rust_expr))
        }
        "title" if tag == "TabItem" => {
            let rust_expr = super::super::component::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".title({}.clone())", rust_expr))
        }
        "title_icon" if tag == "TabItem" => {
            let rust_expr = super::super::component::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".title_icon({})", rust_expr))
        }
        _ => None,
    }
}

/// 事件属性 → builder 方法
///
/// ## TabBar 的 on_click
/// `.on_click(cx.listener(move |this, idx: &usize, _window, cx| { this.<method>(*idx, cx); }))`
/// 用户方法签名约定：`fn method(&mut self, index: usize, cx: &mut Context<Self>)`
///
/// ## Tab 的 on_click
/// 走通用 ClickEvent 路径（与 Button 一致），返回 None 让公共 event_setter 处理。
pub fn event_setter(name: &str, handler: &EventHandler, tag: &str) -> Option<String> {
    if name == "on_click" && tag == "TabBar" {
        let method = match handler {
            EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
            EventHandler::WithArgs(m, _) => m,
        };
        Some(format!(
            ".on_click(cx.listener(move |this, idx: &usize, _window, cx| {{\n                    \
             this.{}(*idx, cx);\n                }}))",
            method
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_tab_bar_variants() {
        assert_eq!(static_setter("underline", "", "TabBar").unwrap(), ".underline()");
        assert_eq!(static_setter("pill", "true", "TabBar").unwrap(), ".pill()");
        assert_eq!(static_setter("flat", "", "TabBar").unwrap(), ".flat()");
        assert_eq!(static_setter("outline", "", "TabBar").unwrap(), ".outline()");
        assert_eq!(static_setter("segmented", "", "TabBar").unwrap(), ".segmented()");
    }

    #[test]
    fn static_setter_tab_variants() {
        assert_eq!(static_setter("underline", "", "Tab").unwrap(), ".underline()");
        assert_eq!(static_setter("pill", "", "Tab").unwrap(), ".pill()");
    }

    #[test]
    fn static_setter_variant_false_returns_none() {
        assert!(static_setter("underline", "false", "TabBar").is_none());
        assert!(static_setter("pill", "0", "TabBar").is_none());
    }

    #[test]
    fn static_setter_tab_bar_menu() {
        assert_eq!(static_setter("menu", "", "TabBar").unwrap(), ".menu(true)");
        assert_eq!(static_setter("menu", "true", "TabBar").unwrap(), ".menu(true)");
        assert_eq!(static_setter("menu", "false", "TabBar").unwrap(), ".menu(false)");
    }

    #[test]
    fn static_setter_menu_only_for_tab_bar() {
        // Tab 不支持 menu
        assert!(static_setter("menu", "true", "Tab").is_none());
    }

    #[test]
    fn static_setter_tab_icon() {
        let code = static_setter("icon", "User", "Tab").unwrap();
        assert_eq!(code, ".icon(rml_ui::IconName::User)");
    }

    #[test]
    fn static_setter_icon_only_for_tab() {
        // TabBar 不支持 icon 属性
        assert!(static_setter("icon", "User", "TabBar").is_none());
    }

    #[test]
    fn static_setter_unknown_returns_none() {
        assert!(static_setter("label", "x", "TabBar").is_none());
        assert!(static_setter("selected_index", "0", "TabBar").is_none());
        assert!(static_setter("foo", "bar", "Tab").is_none());
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
    fn bind_setter_prefix_suffix() {
        let code = bind_setter("prefix", "back_btn", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".prefix(self.back_btn)");
        let code = bind_setter("suffix", "more_btn", &[], &[], "Tab").unwrap();
        assert_eq!(code, ".suffix(self.more_btn)");
    }

    #[test]
    fn bind_setter_selected_index_only_for_tab_bar() {
        assert!(bind_setter("selected_index", "0", &[], &[], "Tab").is_none());
    }

    #[test]
    fn bind_setter_unknown_returns_none() {
        assert!(bind_setter("value", "x", &[], &[], "TabBar").is_none());
        assert!(bind_setter("label", "x", &[], &[], "Tab").is_none());
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
    fn event_setter_tab_on_click_returns_none() {
        // Tab 的 on_click 走通用 ClickEvent 路径，由 component_event_setter 处理
        let handler = EventHandler::Ident("on_click_handler".into());
        assert!(event_setter("on_click", &handler, "Tab").is_none());
    }

    #[test]
    fn event_setter_unknown_event_returns_none() {
        let handler = EventHandler::Ident("h".into());
        assert!(event_setter("onchange", &handler, "TabBar").is_none());
        assert!(event_setter("on_toggle", &handler, "Tab").is_none());
    }
}

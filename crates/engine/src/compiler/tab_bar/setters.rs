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
/// - `closable` / `closable="true"` → `.closable(true)`（Tab / TabItem 共用）
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
        "closable" => {
            let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".closable({})", bool_val))
        }
        "preview" => {
            let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".preview({})", bool_val))
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
/// - `track_scroll={expr}` → `.track_scroll(&<expr>)`（TabBar，ScrollHandle 引用）
/// - `menu={bool expr}` → `.menu(<expr>)`（TabBar）
/// - `closable={bool expr}` → `.closable(<expr>)`（Tab / TabItem 共用）
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
        // track_scroll 接受 &ScrollHandle 引用，不能 clone。
        // 用户在 .rml.rs 中声明 `my_scroll: ScrollHandle` 字段，RML 生成 `.track_scroll(&self.my_scroll)`。
        "track_scroll" if tag == "TabBar" => {
            let rust_expr = super::super::component::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".track_scroll(&{})", rust_expr))
        }
        "closable" => {
            let rust_expr = super::super::component::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".closable({})", rust_expr))
        }
        "preview" => {
            let rust_expr = super::super::component::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".preview({})", rust_expr))
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
/// ## TabBar 的 on_click / on_close / on_close_others / on_promote
/// `.on_close(cx.listener(move |this, idx: &usize, _window, cx| { this.<method>(*idx, cx); }))`
/// 用户方法签名约定：`fn method(&mut self, index: usize, cx: &mut Context<Self>)`
///
/// ## TabBar 的 on_close_all
/// `on_close_all` 回调签名为 `Fn(&mut Window, &mut App)`（2 参，无 event），
/// `cx.listener` 要求 4 参闭包（含 event 参数），无法直接使用。
/// 改用 entity 捕获模式：捕获 `cx.entity()` 句柄，在闭包内 `entity.update(cx, ...)` 回调到视图。
/// 用户方法签名约定：`fn method(&mut self, cx: &mut Context<Self>)`（无 index 参数）
///
/// ## Tab 的 on_click
/// 走通用 ClickEvent 路径（与 Button 一致），返回 None 让公共 event_setter 处理。
pub fn event_setter(name: &str, handler: &EventHandler, tag: &str) -> Option<String> {
    if tag != "TabBar" {
        return None;
    }
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
    };
    match name {
        "on_click" | "on_close" | "on_close_others" | "on_promote" => Some(format!(
            ".{name}(cx.listener(move |this, idx: &usize, _window, cx| {{\n                    \
             this.{method}(*idx, cx);\n                }}))",
            name = name,
            method = method
        )),
        // on_close_all 回调签名为 Fn(&mut Window, &mut App)（无 event 参数），
        // cx.listener 不适用，改用 entity 捕获模式回调到视图方法。
        "on_close_all" => Some(format!(
            ".on_close_all({{\n                    \
             let __entity = cx.entity();\n                    \
             move |_window, cx| {{\n                        \
             __entity.update(cx, |this, cx| {{ this.{method}(cx); }});\n                    \
             }}\n                }})",
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
    fn static_setter_preview() {
        // preview 同 closable，Tab / TabItem 共用
        assert_eq!(static_setter("preview", "", "Tab").unwrap(), ".preview(true)");
        assert_eq!(static_setter("preview", "true", "TabItem").unwrap(), ".preview(true)");
        assert_eq!(static_setter("preview", "false", "Tab").unwrap(), ".preview(false)");
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
        // track_scroll={my_scroll} → .track_scroll(&self.my_scroll)（ScrollHandle 引用）
        let code = bind_setter("track_scroll", "my_scroll", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".track_scroll(&self.my_scroll)");
    }

    #[test]
    fn bind_setter_tab_bar_track_scroll_with_loop_var() {
        // 在 each 循环内：track_scroll={item.scroll} → .track_scroll(&item.scroll)
        let code = bind_setter("track_scroll", "item.scroll", &["item"], &[], "TabBar").unwrap();
        assert_eq!(code, ".track_scroll(&item.scroll)");
    }

    #[test]
    fn bind_setter_track_scroll_only_for_tab_bar() {
        // Tab 不支持 track_scroll
        assert!(bind_setter("track_scroll", "x", &[], &[], "Tab").is_none());
    }

    #[test]
    fn bind_setter_prefix_suffix() {
        let code = bind_setter("prefix", "back_btn", &[], &[], "TabBar").unwrap();
        assert_eq!(code, ".prefix(self.back_btn)");
        let code = bind_setter("suffix", "more_btn", &[], &[], "Tab").unwrap();
        assert_eq!(code, ".suffix(self.more_btn)");
    }

    #[test]
    fn bind_setter_preview() {
        // preview={is_preview} → .preview(self.is_preview)
        let code = bind_setter("preview", "is_preview", &[], &[], "TabItem").unwrap();
        assert_eq!(code, ".preview(self.is_preview)");
        // 在 each 循环内：preview={item.preview} → .preview(item.preview)
        let code = bind_setter("preview", "item.preview", &["item"], &[], "Tab").unwrap();
        assert_eq!(code, ".preview(item.preview)");
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
    fn event_setter_tab_bar_on_close() {
        let handler = EventHandler::Ident("on_tab_close".into());
        let code = event_setter("on_close", &handler, "TabBar").unwrap();
        assert!(code.starts_with(".on_close("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("idx: &usize"));
        assert!(code.contains("this.on_tab_close(*idx, cx)"));
    }

    #[test]
    fn event_setter_tab_bar_on_close_others() {
        let handler = EventHandler::Ident("on_tab_close_others".into());
        let code = event_setter("on_close_others", &handler, "TabBar").unwrap();
        assert!(code.starts_with(".on_close_others("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("idx: &usize"));
        assert!(code.contains("this.on_tab_close_others(*idx, cx)"));
    }

    #[test]
    fn event_setter_tab_bar_on_close_all() {
        // on_close_all 回调签名无 idx 参数：Fn(&mut Window, &mut App)
        // cx.listener 不适用（需 4 参闭包含 event），改用 entity 捕获模式
        let handler = EventHandler::Ident("on_tab_close_all".into());
        let code = event_setter("on_close_all", &handler, "TabBar").unwrap();
        assert!(code.starts_with(".on_close_all("));
        // 不应使用 cx.listener（4 参闭包不兼容 2 参回调）
        assert!(!code.contains("cx.listener"));
        assert!(!code.contains("idx: &usize"));
        // 应使用 entity 捕获 + update 模式
        assert!(code.contains("let __entity = cx.entity()"));
        assert!(code.contains("__entity.update(cx, |this, cx|"));
        assert!(code.contains("this.on_tab_close_all(cx)"));
    }

    #[test]
    fn event_setter_tab_bar_on_promote() {
        // on_promote 模板与 on_close 一致：Fn(&usize, &mut Window, &mut App)
        let handler = EventHandler::Ident("on_tab_promote".into());
        let code = event_setter("on_promote", &handler, "TabBar").unwrap();
        assert!(code.starts_with(".on_promote("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("idx: &usize"));
        assert!(code.contains("this.on_tab_promote(*idx, cx)"));
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
        assert!(event_setter("on_change", &handler, "TabBar").is_none());
        assert!(event_setter("on_toggle", &handler, "Tab").is_none());
    }
}

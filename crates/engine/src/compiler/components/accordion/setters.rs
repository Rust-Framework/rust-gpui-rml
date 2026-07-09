//! Accordion / AccordionItem 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter` /
//! `component_event_setter` 在 tag 为 "Accordion" 或 "AccordionItem" 时委托调用。
//! 未命中返回 None，由公共 setter 回退到通用属性（Sizable、disabled 等）。

use crate::parser::ast::EventHandler;

/// 静态属性 → builder 方法
///
/// - `multiple=""` / `bordered=""` / `open=""` → `.multiple(true)` / `.bordered(true)` / `.open(true)`
/// - `icon="Settings"` → `.icon(rml_ui::IconName::Settings)`
/// - `title="Section 1"` → `.title("Section 1")`
pub fn static_setter(name: &str, value: &str, _tag: &str) -> Option<String> {
    match name {
        "multiple" | "bordered" | "open" => {
            let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".{}({})", name, bool_val))
        }
        "icon" => Some(format!(".icon(rml_ui::IconName::{})", value)),
        "title" => Some(format!(".title({:?})", value)),
        _ => None,
    }
}

/// 绑定属性 → builder 方法
///
/// - `multiple={expr}` → `.multiple(expr)`（bool 表达式）
/// - `title={expr}` → `.title(expr)`（title 接受 impl IntoElement，不加 .clone()）
/// - `icon={expr}` → `.icon(expr)`
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    _tag: &str,
) -> Option<String> {
    match name {
        "multiple" | "bordered" | "open" | "title" | "icon" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".{}({})", name, rust_expr))
        }
        _ => None,
    }
}

/// 事件属性 → builder 方法
///
/// - `on_toggle_click={on_toggle}` →
///   `.on_toggle_click(cx.listener(move |this, open_ixs: &[usize], _window, cx| { this.on_toggle(open_ixs, cx); }))`
///
/// 用户方法签名约定：`fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>)`
pub fn event_setter(name: &str, handler: &EventHandler, _tag: &str) -> Option<String> {
    match name {
        "on_toggle_click" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_toggle_click(cx.listener(move |this, open_ixs: &[usize], _window, cx| {{\n                    \
                 this.{}(open_ixs, cx);\n                }}))",
                method
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_accordion_multiple() {
        let code = static_setter("multiple", "true", "Accordion").unwrap();
        assert_eq!(code, ".multiple(true)");
        let code = static_setter("multiple", "", "Accordion").unwrap();
        assert_eq!(code, ".multiple(true)");
        let code = static_setter("multiple", "false", "Accordion").unwrap();
        assert_eq!(code, ".multiple(false)");
    }

    #[test]
    fn static_setter_accordion_bordered() {
        let code = static_setter("bordered", "true", "Accordion").unwrap();
        assert_eq!(code, ".bordered(true)");
    }

    #[test]
    fn static_setter_accordion_item_open() {
        let code = static_setter("open", "", "AccordionItem").unwrap();
        assert_eq!(code, ".open(true)");
    }

    #[test]
    fn static_setter_accordion_item_icon() {
        let code = static_setter("icon", "Settings", "AccordionItem").unwrap();
        assert_eq!(code, ".icon(rml_ui::IconName::Settings)");
    }

    #[test]
    fn bind_setter_accordion_multiple() {
        let code = bind_setter("multiple", "allow_multi", &[], &[], "Accordion").unwrap();
        assert_eq!(code, ".multiple(self.allow_multi)");
    }

    #[test]
    fn bind_setter_accordion_item_title() {
        // title={section_title} → .title(self.section_title)（不加 .clone()）
        let code = bind_setter("title", "section_title", &[], &[], "AccordionItem").unwrap();
        assert_eq!(code, ".title(self.section_title)");
    }

    #[test]
    fn bind_setter_accordion_item_title_i18n() {
        // title={t("section1")} → .title(t("section1", cx))
        let code = bind_setter("title", "t(\"section1\")", &[], &[], "AccordionItem").unwrap();
        assert!(code.contains(".title("));
        assert!(code.contains("t("));
    }

    #[test]
    fn event_setter_on_toggle_click_accordion() {
        let handler = EventHandler::Ident("on_toggle".into());
        let code = event_setter("on_toggle_click", &handler, "Accordion").unwrap();
        assert!(code.starts_with(".on_toggle_click("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("open_ixs: &[usize]"));
        assert!(code.contains("this.on_toggle"));
    }

    #[test]
    fn event_setter_on_toggle_click_returns_none_for_unknown() {
        let handler = EventHandler::Ident("on_toggle".into());
        assert!(event_setter("on_click", &handler, "Accordion").is_none());
    }

    #[test]
    fn static_setter_returns_none_for_unknown() {
        assert!(static_setter("label", "hello", "Accordion").is_none());
        assert!(static_setter("small", "", "Accordion").is_none());
    }

    #[test]
    fn bind_setter_returns_none_for_unknown() {
        assert!(bind_setter("value", "x", &[], &[], "Accordion").is_none());
        assert!(bind_setter("label", "x", &[], &[], "AccordionItem").is_none());
    }
}

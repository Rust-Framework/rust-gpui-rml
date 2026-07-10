//! Stepper 专用属性 → builder 方法映射。
//!
//! - `direction="vertical"` → `.vertical()`（水平为默认，不生成调用）
//! - `selected_index="2"` → `.selected_index(2usize)`（static）/ `.selected_index(self.field)`（bind）
//! - `text_center=""` → `.text_center(true)`
//! - `on_click` → `.on_click(cx.listener(...))`，闭包参数为 `idx: &usize`
//!
//! StepperItem 专用：
//! - `icon="Check"` → `.icon(rml_ui::Icon::new(rml_ui::IconName::Check))`

use crate::parser::ast::EventHandler;

/// 静态属性 → builder 方法
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    match name {
        // Stepper: direction="vertical" → .vertical()，direction="horizontal" = no-op
        "direction" if tag == "Stepper" => match value {
            "vertical" => Some(".vertical()".to_string()),
            "horizontal" => Some(String::new()),
            _ => None,
        },
        // Stepper: selected_index="2" → .selected_index(2usize)
        "selected_index" if tag == "Stepper" => {
            Some(format!(".selected_index({}usize)", value))
        }
        // Stepper: text_center="" → .text_center(true)
        "text_center" if tag == "Stepper" => {
            let b = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".text_center({})", b))
        }
        // StepperItem: icon="Check" → .icon(rml_ui::Icon::new(rml_ui::IconName::Check))
        "icon" if tag == "StepperItem" => {
            Some(format!(".icon(rml_ui::Icon::new(rml_ui::IconName::{}))", value))
        }
        _ => None,
    }
}

/// 绑定属性 → builder 方法
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    match name {
        // Stepper: selected_index={field} → .selected_index(self.field)
        "selected_index" if tag == "Stepper" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".selected_index({})", rust_expr))
        }
        // Stepper: text_center={bool_expr} → .text_center(self.bool_expr)
        "text_center" if tag == "Stepper" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".text_center({})", rust_expr))
        }
        _ => None,
    }
}

/// 事件属性 → builder 方法
///
/// Stepper 的 on_click 闭包参数是步骤索引（&usize），而非 ClickEvent。
/// 用户方法签名约定：`fn on_step_click(&mut self, idx: &usize, cx: &mut Context<Self>)`
pub fn event_setter(name: &str, handler: &EventHandler, tag: &str) -> Option<String> {
    if tag != "Stepper" {
        return None;
    }
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };
    match name {
        "on_click" => Some(format!(
            ".on_click(cx.listener(move |this, idx: &usize, _window, cx| {{\n                    \
             this.{method}(idx, cx);\n                }}))",
            method = method
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_direction_vertical() {
        assert_eq!(static_setter("direction", "vertical", "Stepper").unwrap(), ".vertical()");
    }

    #[test]
    fn static_setter_direction_horizontal_no_op() {
        assert_eq!(static_setter("direction", "horizontal", "Stepper").unwrap(), "");
    }

    #[test]
    fn static_setter_selected_index() {
        assert_eq!(static_setter("selected_index", "2", "Stepper").unwrap(), ".selected_index(2usize)");
    }

    #[test]
    fn static_setter_text_center_bool() {
        assert_eq!(static_setter("text_center", "", "Stepper").unwrap(), ".text_center(true)");
        assert_eq!(static_setter("text_center", "true", "Stepper").unwrap(), ".text_center(true)");
        assert_eq!(static_setter("text_center", "false", "Stepper").unwrap(), ".text_center(false)");
    }

    #[test]
    fn static_setter_stepper_item_icon() {
        assert_eq!(
            static_setter("icon", "Check", "StepperItem").unwrap(),
            ".icon(rml_ui::Icon::new(rml_ui::IconName::Check))"
        );
    }

    #[test]
    fn static_setter_unknown_returns_none() {
        assert!(static_setter("label", "x", "Stepper").is_none());
        assert!(static_setter("foo", "bar", "Stepper").is_none());
    }

    #[test]
    fn bind_setter_selected_index() {
        let code = bind_setter("selected_index", "current_step", &[], &[], "Stepper").unwrap();
        assert_eq!(code, ".selected_index(self.current_step)");
    }

    #[test]
    fn bind_setter_text_center() {
        let code = bind_setter("text_center", "is_centered", &[], &[], "Stepper").unwrap();
        assert_eq!(code, ".text_center(self.is_centered)");
    }

    #[test]
    fn bind_setter_unknown_returns_none() {
        assert!(bind_setter("value", "x", &[], &[], "Stepper").is_none());
    }

    #[test]
    fn event_setter_on_click() {
        let handler = EventHandler::Ident("on_step_click".into());
        let code = event_setter("on_click", &handler, "Stepper").unwrap();
        assert!(code.starts_with(".on_click("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("idx: &usize"));
        assert!(code.contains("this.on_step_click(idx, cx)"));
    }

    #[test]
    fn event_setter_unknown_returns_none() {
        let handler = EventHandler::Ident("h".into());
        assert!(event_setter("on_change", &handler, "Stepper").is_none());
    }

    #[test]
    fn event_setter_non_stepper_returns_none() {
        let handler = EventHandler::Ident("h".into());
        assert!(event_setter("on_click", &handler, "Button").is_none());
    }
}

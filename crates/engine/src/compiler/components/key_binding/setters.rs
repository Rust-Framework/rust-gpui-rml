//! KeyBinding 专用属性 setter
//!
//! ## 属性映射
//!
//! - `key="Ctrl+S"` (static) → `.key("Ctrl+S")`
//! - `when={cond}` (bind) → `.when(cond)`
//! - `on-press={handler}` (event) → `.on_press({ entity capture })`
//!
//! ## on_press 回调签名
//!
//! `Fn(&mut Window, &mut App)`（2 参，无 event），使用 entity 捕获模式回调到视图方法。
//! 用户方法签名约定：`fn method(&mut self, cx: &mut Context<Self>)`

use crate::parser::ast::EventHandler;

/// KeyBinding 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "key" => Some(format!(".key({:?})", value)),
        _ => None,
    }
}

/// KeyBinding 专用绑定属性 setter
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
) -> Option<String> {
    match name {
        "when" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(
                expr_str, loop_vars, computed,
            );
            Some(format!(".when({})", rust_expr))
        }
        _ => None,
    }
}

/// KeyBinding 专用事件属性 setter
///
/// `on_press` 回调签名为 `Fn(&mut Window, &mut App)`（2 参，无 event），
/// 使用 entity 捕获模式回调到视图方法。
pub fn event_setter(name: &str, handler: &EventHandler) -> Option<String> {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };
    match name {
        "on_press" => Some(format!(
            ".on_press({{\n                    \
             let __entity = cx.entity();\n                    \
             move |_window, cx| {{\n                        \
             __entity.update(cx, |this, cx| {{ this.{}(cx); }});\n                    \
             }}\n                }})",
            method
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_key() {
        assert_eq!(
            static_setter("key", "Ctrl+S"),
            Some(".key(\"Ctrl+S\")".to_string())
        );
    }

    #[test]
    fn static_setter_unknown() {
        assert!(static_setter("unknown", "x").is_none());
    }

    #[test]
    fn bind_setter_when() {
        let s = bind_setter("when", "is_active", &[], &[]).unwrap();
        assert_eq!(s, ".when(self.is_active)");
    }

    #[test]
    fn bind_setter_unknown() {
        assert!(bind_setter("unknown", "x", &[], &[]).is_none());
    }

    #[test]
    fn event_setter_on_press() {
        let handler = EventHandler::Ident("handle_save".into());
        let s = event_setter("on_press", &handler).unwrap();
        assert!(s.contains(".on_press("));
        assert!(s.contains("let __entity = cx.entity()"));
        assert!(s.contains("this.handle_save(cx)"));
    }

    #[test]
    fn event_setter_unknown() {
        let handler = EventHandler::Ident("handle".into());
        assert!(event_setter("on_click", &handler).is_none());
    }
}

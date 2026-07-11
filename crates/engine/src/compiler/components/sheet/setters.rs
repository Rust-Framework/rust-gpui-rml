//! Sheet 专用属性 setter
//!
//! ## 属性映射
//!
//! - `title="标题"` → `.title("标题")`（impl IntoElement，&str 自动转换）
//! - `footer="页脚"` → `.footer("页脚")`
//! - `size="350px"` → `.size(gpui::px(350.0))`
//! - `size="50%"` → `.size(gpui::relative(0.5))`
//! - `resizable="false"` → `.resizable(false)`
//! - `overlay="false"` → `.overlay(false)`
//! - `overlay_closable="false"` → `.overlay_closable(false)`
//! - `on_close={handler}` → `.on_close(cx.listener(...))`（事件 setter）

use crate::parser::ast::EventHandler;

/// Sheet 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "title" => Some(format!(".title({:?})", value)),
        "footer" => Some(format!(".footer({:?})", value)),
        "size" => parse_size(value),
        "resizable" => parse_bool(name, value),
        "overlay" => parse_bool(name, value),
        "overlay_closable" => parse_bool(name, value),
        _ => None,
    }
}

/// 解析 size 属性：px / % / 裸数字
fn parse_size(value: &str) -> Option<String> {
    if let Some(n) = value.strip_suffix("px") {
        let n: f32 = n.parse().ok()?;
        Some(format!(".size(gpui::px({:?}))", n))
    } else if let Some(n) = value.strip_suffix('%') {
        let n: f32 = n.parse().ok()?;
        Some(format!(".size(gpui::relative({:?}))", n / 100.0))
    } else {
        let n: f32 = value.parse().ok()?;
        Some(format!(".size(gpui::px({:?}))", n))
    }
}

/// 解析布尔属性：false → `.method(false)`，true → 空字符串（默认值，不生成调用）
fn parse_bool(name: &str, value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("false") {
        Some(format!(".{}(false)", name))
    } else {
        Some(String::new())
    }
}

/// Sheet 专用事件 setter
///
/// `on_close` 签名为 `impl Fn(&ClickEvent, &mut Window, &mut App)`，
/// 与 Alert 的 `on_close` 一致，使用 `cx.listener()` 桥接。
pub fn event_setter(name: &str, handler: &EventHandler) -> Option<String> {
    if name != "on_close" {
        return None;
    }

    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };

    match handler {
        EventHandler::Ident(_) | EventHandler::MethodName(_) => Some(format!(
            ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
             let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                    \
             this.{}(&rml_ev, cx);\n                }}))",
            method
        )),
        EventHandler::ClosureField(_) => None,
        EventHandler::WithArgs(_, args) if args.is_empty() => Some(format!(
            ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
             let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                    \
             this.{}(&rml_ev, cx);\n                }}))",
            method
        )),
        EventHandler::WithArgs(_, args) => {
            let arg = &args[0];
            Some(format!(
                ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
                 let p0 = {}.clone();\n                    \
                 let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                    \
                 this.{}(p0, &rml_ev, cx);\n                }}))",
                arg, method
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::EventHandler;

    #[test]
    fn static_setter_title() {
        assert_eq!(
            static_setter("title", "设置面板"),
            Some(".title(\"设置面板\")".to_string())
        );
    }

    #[test]
    fn static_setter_footer() {
        assert_eq!(
            static_setter("footer", "底部操作栏"),
            Some(".footer(\"底部操作栏\")".to_string())
        );
    }

    #[test]
    fn static_setter_size_px() {
        assert_eq!(
            static_setter("size", "350px"),
            Some(".size(gpui::px(350.0))".to_string())
        );
    }

    #[test]
    fn static_setter_size_percent() {
        assert_eq!(
            static_setter("size", "50%"),
            Some(".size(gpui::relative(0.5))".to_string())
        );
    }

    #[test]
    fn static_setter_size_bare_number() {
        assert_eq!(
            static_setter("size", "400"),
            Some(".size(gpui::px(400.0))".to_string())
        );
    }

    #[test]
    fn static_setter_resizable_false() {
        assert_eq!(
            static_setter("resizable", "false"),
            Some(".resizable(false)".to_string())
        );
    }

    #[test]
    fn static_setter_resizable_true_no_op() {
        let s = static_setter("resizable", "true").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_overlay_false() {
        assert_eq!(
            static_setter("overlay", "false"),
            Some(".overlay(false)".to_string())
        );
    }

    #[test]
    fn static_setter_overlay_closable_false() {
        assert_eq!(
            static_setter("overlay_closable", "false"),
            Some(".overlay_closable(false)".to_string())
        );
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }

    #[test]
    fn event_setter_on_close_ident() {
        let handler = EventHandler::Ident("on_sheet_close".into());
        let code = event_setter("on_close", &handler).unwrap();
        assert!(code.starts_with(".on_close(cx.listener("));
        assert!(code.contains("this.on_sheet_close"));
    }

    #[test]
    fn event_setter_on_close_method_name() {
        let handler = EventHandler::MethodName("handle_close".into());
        let code = event_setter("on_close", &handler).unwrap();
        assert!(code.contains("this.handle_close"));
    }

    #[test]
    fn event_setter_unknown() {
        let handler = EventHandler::Ident("handler".into());
        assert!(event_setter("on_click", &handler).is_none());
    }
}

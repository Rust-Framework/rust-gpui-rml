//! AlertDialog 专用属性 setter
//!
//! ## 与 Dialog 的区别
//!
//! AlertDialog 基于 Dialog 封装，但默认值和 API 不同：
//! - `close_button` 默认 `false`（Dialog 默认 `true`）
//! - `overlay_closable` 默认 `false`（Dialog 默认 `true`）
//! - 提供 `.description()` 便捷方法（Dialog 无）
//! - 提供 `.confirm()` 方法（显示取消按钮，Dialog 无）
//! - 提供 `.show_cancel(bool)` 方法（Dialog 无）
//! - footer 按钮居中对齐（Dialog 右对齐）
//!
//! ## 属性映射
//!
//! - `title="标题"` → `.title("标题")`
//! - `description="描述"` → `.description("描述")`（AlertDialog 专属）
//! - `width="420px"` → `.width(gpui::px(420.0))`
//! - `confirm` → `.confirm()`（布尔属性，存在即调用，显示取消按钮）
//! - `show_cancel="true"` → `.show_cancel(true)`
//! - `overlay_closable="true"` → `.overlay_closable(true)`（AlertDialog 默认 false，需显式开启）
//! - `close_button="true"` → `.close_button(true)`（AlertDialog 默认 false，需显式开启）
//! - `keyboard="false"` → `.keyboard(false)`
//! - `on_close={handler}` → `.on_close(cx.listener(...))`（同 Dialog）
//! - `on_ok={handler}` → `.on_ok({ entity 捕获闭包，返回 handler 方法的 bool 返回值 })`（同 Dialog）
//! - `on_cancel={handler}` → `.on_cancel({ entity 捕获闭包，返回 handler 方法的 bool 返回值 })`（同 Dialog）

use crate::parser::ast::EventHandler;

/// AlertDialog 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "title" => Some(format!(".title({:?})", value)),
        "description" => Some(format!(".description({:?})", value)),
        "width" => parse_width(value),
        // confirm 为布尔属性：存在即调用 .confirm()，忽略值
        "confirm" => Some(".confirm()".to_string()),
        // show_cancel：true → .show_cancel(true)，false → 空字符串（默认不显示）
        "show_cancel" => parse_bool_enabled(name, value),
        // overlay_closable：AlertDialog 默认 false，显式 true 开启
        "overlay_closable" => parse_bool_enabled(name, value),
        // close_button：AlertDialog 默认 false，显式 true 开启
        "close_button" => parse_bool_enabled(name, value),
        // keyboard：默认 true，false 关闭
        "keyboard" => parse_bool_disabled(name, value),
        _ => None,
    }
}

/// 解析 width 属性：px / 裸数字
fn parse_width(value: &str) -> Option<String> {
    if let Some(n) = value.strip_suffix("px") {
        let n: f32 = n.parse().ok()?;
        Some(format!(".width(gpui::px({:?}))", n))
    } else {
        let n: f32 = value.parse().ok()?;
        Some(format!(".width(gpui::px({:?}))", n))
    }
}

/// 布尔属性：默认 false，显式 "true" 开启 → .method(true)
fn parse_bool_enabled(name: &str, value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("true") {
        Some(format!(".{}(true)", name))
    } else {
        Some(String::new())
    }
}

/// 布尔属性：默认 true，显式 "false" 关闭 → .method(false)
fn parse_bool_disabled(name: &str, value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("false") {
        Some(format!(".{}(false)", name))
    } else {
        Some(String::new())
    }
}

/// AlertDialog 专用事件 setter（与 Dialog 一致）
pub fn event_setter(name: &str, handler: &EventHandler) -> Option<String> {
    match name {
        "on_close" => Some(on_close_setter(handler)),
        "on_ok" => Some(bool_event_setter("on_ok", handler)),
        "on_cancel" => Some(bool_event_setter("on_cancel", handler)),
        _ => None,
    }
}

/// on_close setter：使用 cx.listener() 模式（与 Dialog 一致）
fn on_close_setter(handler: &EventHandler) -> String {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };

    match handler {
        EventHandler::Ident(_) | EventHandler::MethodName(_) => format!(
            ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
             let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                    \
             this.{}(&rml_ev, cx);\n                }}))",
            method
        ),
        EventHandler::ClosureField(_) => String::new(),
        EventHandler::WithArgs(_, args) if args.is_empty() => format!(
            ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
             let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                    \
             this.{}(&rml_ev, cx);\n                }}))",
            method
        ),
        EventHandler::WithArgs(_, args) => {
            let arg = &args[0];
            format!(
                ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
                 let p0 = {}.clone();\n                    \
                 let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                    \
                 this.{}(p0, &rml_ev, cx);\n                }}))",
                arg, method
            )
        }
    }
}

/// on_ok / on_cancel setter：手动 entity 捕获闭包，传递 handler 方法的 bool 返回值（与 Dialog 一致）
fn bool_event_setter(setter_name: &str, handler: &EventHandler) -> String {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };

    match handler {
        EventHandler::Ident(_) | EventHandler::MethodName(_) => format!(
            ".{}({{\n                    \
             let entity = cx.entity();\n                    \
             move |_ev: &gpui::ClickEvent, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n                        \
             let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                        \
             entity.update(cx, |this, cx| {{\n                            \
             this.{}(&rml_ev, cx)\n                        }})\n                    \
             }}\n                }})",
            setter_name, method
        ),
        EventHandler::ClosureField(_) => String::new(),
        EventHandler::WithArgs(_, args) if args.is_empty() => format!(
            ".{}({{\n                    \
             let entity = cx.entity();\n                    \
             move |_ev: &gpui::ClickEvent, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n                        \
             let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                        \
             entity.update(cx, |this, cx| {{\n                            \
             this.{}(&rml_ev, cx)\n                        }})\n                    \
             }}\n                }})",
            setter_name, method
        ),
        EventHandler::WithArgs(_, args) => {
            let arg = &args[0];
            format!(
                ".{}({{\n                    \
                 let entity = cx.entity();\n                    \
                 move |_ev: &gpui::ClickEvent, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n                        \
                 let p0 = {}.clone();\n                        \
                 let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n                        \
                 entity.update(cx, |this, cx| {{\n                            \
                 this.{}(p0, &rml_ev, cx)\n                        }})\n                    \
                 }}\n                }})",
                setter_name, arg, method
            )
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
            static_setter("title", "确认删除"),
            Some(".title(\"确认删除\")".to_string())
        );
    }

    #[test]
    fn static_setter_description() {
        assert_eq!(
            static_setter("description", "此操作不可撤销"),
            Some(".description(\"此操作不可撤销\")".to_string())
        );
    }

    #[test]
    fn static_setter_width_px() {
        assert_eq!(
            static_setter("width", "420px"),
            Some(".width(gpui::px(420.0))".to_string())
        );
    }

    #[test]
    fn static_setter_width_bare_number() {
        assert_eq!(
            static_setter("width", "500"),
            Some(".width(gpui::px(500.0))".to_string())
        );
    }

    #[test]
    fn static_setter_confirm() {
        assert_eq!(static_setter("confirm", ""), Some(".confirm()".to_string()));
        assert_eq!(
            static_setter("confirm", "true"),
            Some(".confirm()".to_string())
        );
    }

    #[test]
    fn static_setter_show_cancel_true() {
        assert_eq!(
            static_setter("show_cancel", "true"),
            Some(".show_cancel(true)".to_string())
        );
    }

    #[test]
    fn static_setter_show_cancel_false_no_op() {
        let s = static_setter("show_cancel", "false").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_overlay_closable_true() {
        assert_eq!(
            static_setter("overlay_closable", "true"),
            Some(".overlay_closable(true)".to_string())
        );
    }

    #[test]
    fn static_setter_close_button_true() {
        assert_eq!(
            static_setter("close_button", "true"),
            Some(".close_button(true)".to_string())
        );
    }

    #[test]
    fn static_setter_keyboard_false() {
        assert_eq!(
            static_setter("keyboard", "false"),
            Some(".keyboard(false)".to_string())
        );
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }

    #[test]
    fn event_setter_on_close_ident() {
        let handler = EventHandler::Ident("on_alert_close".into());
        let code = event_setter("on_close", &handler).unwrap();
        assert!(code.starts_with(".on_close(cx.listener("));
        assert!(code.contains("this.on_alert_close"));
    }

    #[test]
    fn event_setter_on_ok_ident() {
        let handler = EventHandler::Ident("handle_ok".into());
        let code = event_setter("on_ok", &handler).unwrap();
        assert!(code.starts_with(".on_ok({"));
        assert!(code.contains("let entity = cx.entity();"));
        assert!(code.contains("this.handle_ok"));
        assert!(!code.contains("\n             true"));
    }

    #[test]
    fn event_setter_on_cancel_ident() {
        let handler = EventHandler::Ident("handle_cancel".into());
        let code = event_setter("on_cancel", &handler).unwrap();
        assert!(code.starts_with(".on_cancel({"));
        assert!(code.contains("this.handle_cancel"));
        assert!(!code.contains("\n             true"));
    }

    #[test]
    fn event_setter_unknown() {
        let handler = EventHandler::Ident("handler".into());
        assert!(event_setter("on_click", &handler).is_none());
    }
}

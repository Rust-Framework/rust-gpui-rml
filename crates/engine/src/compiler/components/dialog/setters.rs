//! Dialog 专用属性 setter
//!
//! ## 属性映射
//!
//! - `title="标题"` → `.title("标题")`（impl IntoElement，&str 自动转换）
//! - `footer="页脚"` → `.footer("页脚")`
//! - `width="500px"` → `.width(gpui::px(500.0))`
//! - `width="500"` → `.width(gpui::px(500.0))`（裸数字按 px 处理）
//! - `overlay="false"` → `.overlay(false)`
//! - `overlay_closable="false"` → `.overlay_closable(false)`
//! - `close_button="false"` → `.close_button(false)`
//! - `keyboard="false"` → `.keyboard(false)`
//! - `on_close={handler}` → `.on_close(cx.listener(...))`（事件 setter，同 Sheet）
//! - `on_ok={handler}` → `.on_ok({ entity 捕获闭包，返回 true })`（bool 返回值，不能用 cx.listener）
//! - `on_cancel={handler}` → `.on_cancel({ entity 捕获闭包，返回 true })`（bool 返回值，不能用 cx.listener）

use crate::parser::ast::EventHandler;

/// Dialog 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "title" => Some(format!(".title({:?})", value)),
        "footer" => Some(format!(".footer({:?})", value)),
        "width" => parse_width(value),
        "overlay" => parse_bool(name, value),
        "overlay_closable" => parse_bool(name, value),
        "close_button" => parse_bool(name, value),
        "keyboard" => parse_bool(name, value),
        _ => None,
    }
}

/// 解析 width 属性：px / 裸数字（Dialog width 接收 impl Into<Pixels>，不支持 %）
fn parse_width(value: &str) -> Option<String> {
    if let Some(n) = value.strip_suffix("px") {
        let n: f32 = n.parse().ok()?;
        Some(format!(".width(gpui::px({:?}))", n))
    } else {
        let n: f32 = value.parse().ok()?;
        Some(format!(".width(gpui::px({:?}))", n))
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

/// Dialog 专用事件 setter
///
/// - `on_close`：签名为 `impl Fn(&ClickEvent, &mut Window, &mut App)`，使用 `cx.listener()` 桥接（同 Sheet）
/// - `on_ok` / `on_cancel`：签名为 `impl Fn(&ClickEvent, &mut Window, &mut App) -> bool`，
///   `cx.listener()` 无法直接使用（不返回 bool），改用手动 entity 捕获闭包，固定返回 `true`
pub fn event_setter(name: &str, handler: &EventHandler) -> Option<String> {
    match name {
        "on_close" => Some(on_close_setter(handler)),
        "on_ok" => Some(bool_event_setter("on_ok", handler)),
        "on_cancel" => Some(bool_event_setter("on_cancel", handler)),
        _ => None,
    }
}

/// on_close setter：使用 cx.listener() 模式（与 Sheet 一致）
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

/// on_ok / on_cancel setter：手动 entity 捕获闭包，返回 true
///
/// 签名为 `Fn(&ClickEvent, &mut Window, &mut App) -> bool`，
/// `cx.listener()` 产生 `Fn(&mut T, &ClickEvent, &mut Window, &mut Context<T>)`（无返回值），
/// 无法适配。改用 `cx.entity()` 捕获 entity，在闭包内 `entity.update(cx, |this, cx| ...)` 调用方法，
/// 固定返回 `true`（声明式 API 默认关闭对话框）。
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
             this.{}(&rml_ev, cx);\n                        }});\n                        \
             true\n                    \
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
             this.{}(&rml_ev, cx);\n                        }});\n                        \
             true\n                    \
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
                 this.{}(p0, &rml_ev, cx);\n                        }});\n                        \
                 true\n                    \
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
    fn static_setter_footer() {
        assert_eq!(
            static_setter("footer", "操作区域"),
            Some(".footer(\"操作区域\")".to_string())
        );
    }

    #[test]
    fn static_setter_width_px() {
        assert_eq!(
            static_setter("width", "500px"),
            Some(".width(gpui::px(500.0))".to_string())
        );
    }

    #[test]
    fn static_setter_width_bare_number() {
        assert_eq!(
            static_setter("width", "600"),
            Some(".width(gpui::px(600.0))".to_string())
        );
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
    fn static_setter_close_button_false() {
        assert_eq!(
            static_setter("close_button", "false"),
            Some(".close_button(false)".to_string())
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
    fn static_setter_overlay_true_no_op() {
        let s = static_setter("overlay", "true").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }

    #[test]
    fn event_setter_on_close_ident() {
        let handler = EventHandler::Ident("on_dialog_close".into());
        let code = event_setter("on_close", &handler).unwrap();
        assert!(code.starts_with(".on_close(cx.listener("));
        assert!(code.contains("this.on_dialog_close"));
    }

    #[test]
    fn event_setter_on_ok_ident() {
        let handler = EventHandler::Ident("handle_ok".into());
        let code = event_setter("on_ok", &handler).unwrap();
        assert!(code.starts_with(".on_ok({"));
        assert!(code.contains("let entity = cx.entity();"));
        assert!(code.contains("entity.update(cx"));
        assert!(code.contains("this.handle_ok"));
        assert!(code.contains("true"));
    }

    #[test]
    fn event_setter_on_cancel_ident() {
        let handler = EventHandler::Ident("handle_cancel".into());
        let code = event_setter("on_cancel", &handler).unwrap();
        assert!(code.starts_with(".on_cancel({"));
        assert!(code.contains("let entity = cx.entity();"));
        assert!(code.contains("this.handle_cancel"));
        assert!(code.contains("true"));
    }

    #[test]
    fn event_setter_on_ok_with_args() {
        let handler = EventHandler::WithArgs("handle_ok".into(), vec!["user_id".into()]);
        let code = event_setter("on_ok", &handler).unwrap();
        assert!(code.contains("let p0 = user_id.clone();"));
        assert!(code.contains("this.handle_ok(p0"));
    }

    #[test]
    fn event_setter_unknown() {
        let handler = EventHandler::Ident("handler".into());
        assert!(event_setter("on_click", &handler).is_none());
    }
}

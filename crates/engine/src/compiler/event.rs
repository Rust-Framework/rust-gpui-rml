//! 事件绑定代码生成
//!
//! 将 RML 事件名（如 `onclick`）映射到 GPUI 元素方法（如 `.on_click(...)`），
//! 并生成带 `cx.listener` 包装的闭包代码。
//!
//! ## 事件分类
//!
//! | 类型 | 示例 | GPUI 回调签名 | 处理方式 |
//! |------|------|---------------|---------|
//! | 标准事件 | onclick, onkeydown | `Fn(&EventType, &mut Window, &mut App)` | `apply_event` |
//! | 悬停事件 | onhover, onmouseenter | `Fn(&bool, &mut Window, &mut App)` | `apply_hover_event` |
//! | 不支持 | oninput, onchange | GPUI 元素无对应方法 | 返回空字符串 |
//!
//! 详见文档 §5.4 事件绑定与 §10.6 代码生成。

use crate::compiler::CodegenCtx;
use crate::parser::ast::EventHandler;

/// 事件名 → (GPUI 事件类型, GPUI on_* 方法名, 转换函数路径)
///
/// 返回 (event_type, on_method, convert_expr):
/// - `event_type`：GPUI 事件类型名，用于闭包参数标注
/// - `on_method`：GPUI StatefulInteractiveElement 方法名
/// - `convert_expr`：调用 rml_convert 的表达式，传入 GPUI 事件引用，返回 RML 事件对象
pub fn event_binding(name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match name {
        // 鼠标事件
        "onclick" => Some(("gpui::ClickEvent", "on_click", "rml_convert::from_gpui_click(ev)")),
        "onauxclick" => Some(("gpui::ClickEvent", "on_aux_click", "rml_convert::from_gpui_click(ev)")),
        "onmousedown" => Some((
            "gpui::MouseDownEvent",
            "on_mouse_down",
            "rml_convert::from_gpui_mouse_down(ev)",
        )),
        "onmouseup" => Some((
            "gpui::MouseUpEvent",
            "on_mouse_up",
            "rml_convert::from_gpui_mouse_up(ev)",
        )),
        "onmousemove" => Some((
            "gpui::MouseMoveEvent",
            "on_mouse_move",
            "rml_convert::from_gpui_mouse_move(ev)",
        )),
        "onwheel" => Some((
            "gpui::ScrollWheelEvent",
            "on_scroll_wheel",
            "rml_convert::from_gpui_scroll_wheel(ev)",
        )),
        // 键盘事件
        "onkeydown" => Some((
            "gpui::KeyDownEvent",
            "on_key_down",
            "rml_convert::from_gpui_key_down(ev)",
        )),
        "onkeyup" => Some((
            "gpui::KeyUpEvent",
            "on_key_up",
            "rml_convert::from_gpui_key_up(ev)",
        )),
        // 以下事件 GPUI 不直接提供对应类型，Phase B-2 由 codegen 直接构造 RML 事件
        // 暂不绑定（返回 None）
        "oninput" | "onchange" | "onsubmit" | "onfocus" | "onblur" | "onload" | "onresize" | "onscroll" => None,
        _ => None,
    }
}

/// 判断事件是否为悬停类型（需要特殊处理 &bool 回调）
pub fn is_hover_event(name: &str) -> bool {
    matches!(name, "onhover" | "onmouseenter" | "onmouseleave")
}

/// 生成事件绑定代码
///
/// 标准事件：`.on_click(cx.listener(move |this, ev: &gpui::ClickEvent, _window, cx| { ... }))`
/// 悬停事件：委托给 `apply_hover_event`
/// 不支持事件：返回空字符串
pub fn apply_event(name: &str, handler: &EventHandler, _ctx: &CodegenCtx) -> String {
    // onhover 特殊处理：GPUI on_hover 回调接收 &bool 而非 &EventType
    if is_hover_event(name) {
        return apply_hover_event(name, handler);
    }

    // 未知事件名：跳过
    let (gpui_type, on_method, convert_expr) = match event_binding(name) {
        Some(binding) => binding,
        None => return String::new(),
    };

    match handler {
        EventHandler::Ident(method) | EventHandler::MethodName(method) => {
            // .on_click(cx.listener(move |this, ev: &gpui::ClickEvent, _window, cx| {
            //     let rml_ev = rml_convert::from_gpui_click(ev);
            //     this.increment(&rml_ev, cx);
            //     if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }
            // }))
            format!(
                ".{}(cx.listener(move |this, ev: &{}, _window, cx| {{\n                    \
                 let rml_ev = {};\n                    this.{}(&rml_ev, cx);\n                    \
                 if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n                }}))",
                on_method, gpui_type, convert_expr, method
            )
        }
        EventHandler::WithArgs(method, args) => {
            if args.is_empty() {
                format!(
                    ".{}(cx.listener(move |this, ev: &{}, _window, cx| {{\n                    \
                     let rml_ev = {};\n                    this.{}(&rml_ev, cx);\n                    \
                     if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n                }}))",
                    on_method, gpui_type, convert_expr, method
                )
            } else {
                // Phase B-1 简化：仅支持单参数
                let arg = &args[0];
                format!(
                    ".{}(cx.listener(move |this, ev: &{}, _window, cx| {{\n                    \
                     let p0 = {}.clone();\n                    let rml_ev = {};\n                    \
                     this.{}(p0, &rml_ev, cx);\n                    \
                     if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n                }}))",
                    on_method, gpui_type, arg, convert_expr, method
                )
            }
        }
    }
}

/// 生成 onhover/onmouseenter/onmouseleave 事件绑定代码
///
/// GPUI `on_hover` 回调签名为 `Fn(&bool, &mut Window, &mut App)`，
/// `&bool` 为 true 表示进入，false 表示离开。
/// - `onhover`：进入和离开都触发
/// - `onmouseenter`：仅 `is_hovering == true` 时触发
/// - `onmouseleave`：仅 `is_hovering == false` 时触发
pub fn apply_hover_event(name: &str, handler: &EventHandler) -> String {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
    };

    // onmouseenter/onmouseleave 需要条件过滤
    let condition = match name {
        "onmouseenter" => Some("is_hovering"),
        "onmouseleave" => Some("!is_hovering"),
        _ => None,
    };

    let body = if let Some(cond) = condition {
        format!(
            "if {} {{\n                    \
             let rml_ev = rml_convert::from_gpui_hover(&is_hovering);\n                    \
             this.{}(&rml_ev, cx);\n                }}",
            cond, method
        )
    } else {
        format!(
            "let rml_ev = rml_convert::from_gpui_hover(&is_hovering);\n                    \
             this.{}(&rml_ev, cx);",
            method
        )
    };

    format!(
        ".on_hover(cx.listener(move |this, is_hovering: &bool, _window, cx| {{\n                    \
         {}\n                }}))",
        body
    )
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    // ─── event_binding ───

    #[test]
    fn event_binding_onclick() {
        let (ty, method, convert) = event_binding("onclick").unwrap();
        assert_eq!(ty, "gpui::ClickEvent");
        assert_eq!(method, "on_click");
        assert!(convert.contains("from_gpui_click"));
    }

    #[test]
    fn event_binding_onauxclick() {
        let (_, method, _) = event_binding("onauxclick").unwrap();
        assert_eq!(method, "on_aux_click");
    }

    #[test]
    fn event_binding_onmousedown() {
        let (ty, method, convert) = event_binding("onmousedown").unwrap();
        assert_eq!(ty, "gpui::MouseDownEvent");
        assert_eq!(method, "on_mouse_down");
        assert!(convert.contains("from_gpui_mouse_down"));
    }

    #[test]
    fn event_binding_onmouseup() {
        let (ty, method, _) = event_binding("onmouseup").unwrap();
        assert_eq!(ty, "gpui::MouseUpEvent");
        assert_eq!(method, "on_mouse_up");
    }

    #[test]
    fn event_binding_onmousemove() {
        let (ty, method, _) = event_binding("onmousemove").unwrap();
        assert_eq!(ty, "gpui::MouseMoveEvent");
        assert_eq!(method, "on_mouse_move");
    }

    #[test]
    fn event_binding_onwheel() {
        let (ty, method, _) = event_binding("onwheel").unwrap();
        assert_eq!(ty, "gpui::ScrollWheelEvent");
        assert_eq!(method, "on_scroll_wheel");
    }

    #[test]
    fn event_binding_onkeydown() {
        let (ty, method, _) = event_binding("onkeydown").unwrap();
        assert_eq!(ty, "gpui::KeyDownEvent");
        assert_eq!(method, "on_key_down");
    }

    #[test]
    fn event_binding_onkeyup() {
        let (ty, method, _) = event_binding("onkeyup").unwrap();
        assert_eq!(ty, "gpui::KeyUpEvent");
        assert_eq!(method, "on_key_up");
    }

    #[test]
    fn event_binding_unsupported_returns_none() {
        // GPUI 元素级不支持的事件
        assert!(event_binding("oninput").is_none());
        assert!(event_binding("onchange").is_none());
        assert!(event_binding("onsubmit").is_none());
        assert!(event_binding("onfocus").is_none());
        assert!(event_binding("onblur").is_none());
        assert!(event_binding("onload").is_none());
        assert!(event_binding("onresize").is_none());
        assert!(event_binding("onscroll").is_none());
    }

    #[test]
    fn event_binding_unknown_returns_none() {
        assert!(event_binding("oncustom").is_none());
        assert!(event_binding("").is_none());
        assert!(event_binding("click").is_none()); // 缺少 on 前缀
    }

    #[test]
    fn event_binding_hover_returns_none() {
        // 悬停事件由 is_hover_event 单独处理，不在 event_binding 中
        assert!(event_binding("onhover").is_none());
        assert!(event_binding("onmouseenter").is_none());
        assert!(event_binding("onmouseleave").is_none());
    }

    // ─── is_hover_event ───

    #[test]
    fn is_hover_event_recognizes_all_three() {
        assert!(is_hover_event("onhover"));
        assert!(is_hover_event("onmouseenter"));
        assert!(is_hover_event("onmouseleave"));
    }

    #[test]
    fn is_hover_event_rejects_non_hover() {
        assert!(!is_hover_event("onclick"));
        assert!(!is_hover_event("onmousedown"));
        assert!(!is_hover_event(""));
        assert!(!is_hover_event("hover")); // 缺少 on 前缀
    }

    // ─── apply_event：标准事件 ───

    #[test]
    fn apply_event_onclick_ident() {
        let handler = EventHandler::Ident("increment".into());
        let code = apply_event("onclick", &handler, &ctx());
        assert!(code.starts_with(".on_click("));
        assert!(code.contains("gpui::ClickEvent"));
        assert!(code.contains("from_gpui_click(ev)"));
        assert!(code.contains("this.increment"));
        assert!(code.contains("cx.listener"));
    }

    #[test]
    fn apply_event_onclick_method_name() {
        let handler = EventHandler::MethodName("handle_click".into());
        let code = apply_event("onclick", &handler, &ctx());
        assert!(code.contains("this.handle_click"));
    }

    #[test]
    fn apply_event_onkeydown() {
        let handler = EventHandler::Ident("on_key".into());
        let code = apply_event("onkeydown", &handler, &ctx());
        assert!(code.starts_with(".on_key_down("));
        assert!(code.contains("gpui::KeyDownEvent"));
        assert!(code.contains("from_gpui_key_down(ev)"));
        assert!(code.contains("this.on_key"));
    }

    #[test]
    fn apply_event_with_args_empty() {
        // WithArgs 但参数为空，等同于无参数
        let handler = EventHandler::WithArgs("increment".into(), vec![]);
        let code = apply_event("onclick", &handler, &ctx());
        assert!(code.contains("this.increment"));
        assert!(!code.contains("p0"));
    }

    #[test]
    fn apply_event_with_args_single() {
        let handler = EventHandler::WithArgs("set_value".into(), vec!["42".into()]);
        let code = apply_event("onclick", &handler, &ctx());
        assert!(code.contains("let p0 = 42.clone();"));
        assert!(code.contains("this.set_value(p0,"));
    }

    #[test]
    fn apply_event_unsupported_returns_empty() {
        let handler = EventHandler::Ident("handler".into());
        assert_eq!(apply_event("oninput", &handler, &ctx()), "");
        assert_eq!(apply_event("onblur", &handler, &ctx()), "");
        assert_eq!(apply_event("oncustom", &handler, &ctx()), "");
    }

    // ─── stop_propagation 检查注入 ───

    #[test]
    fn stop_propagation_check_injected_ident() {
        // Ident handler 的闭包末尾应注入 stop_propagation 检查
        let handler = EventHandler::Ident("increment".into());
        let code = apply_event("onclick", &handler, &ctx());
        assert!(
            code.contains("if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }"),
            "Ident handler 应注入 stop_propagation 检查，实际：\n{}",
            code
        );
    }

    #[test]
    fn stop_propagation_check_injected_method_name() {
        let handler = EventHandler::MethodName("handle_click".into());
        let code = apply_event("onclick", &handler, &ctx());
        assert!(
            code.contains("if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }"),
            "MethodName handler 应注入 stop_propagation 检查"
        );
    }

    #[test]
    fn stop_propagation_check_injected_with_args_empty() {
        let handler = EventHandler::WithArgs("increment".into(), vec![]);
        let code = apply_event("onclick", &handler, &ctx());
        assert!(
            code.contains("if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }"),
            "WithArgs(空) handler 应注入 stop_propagation 检查"
        );
    }

    #[test]
    fn stop_propagation_check_injected_with_args_single() {
        let handler = EventHandler::WithArgs("set_value".into(), vec!["42".into()]);
        let code = apply_event("onclick", &handler, &ctx());
        assert!(
            code.contains("if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }"),
            "WithArgs(单参) handler 应注入 stop_propagation 检查"
        );
    }

    // ─── apply_hover_event ───

    #[test]
    fn apply_hover_event_onhover() {
        let handler = EventHandler::Ident("on_hover_change".into());
        let code = apply_hover_event("onhover", &handler);
        assert!(code.starts_with(".on_hover("));
        assert!(code.contains("is_hovering: &bool"));
        assert!(code.contains("from_gpui_hover"));
        assert!(code.contains("this.on_hover_change"));
        // onhover 不应该有条件过滤
        assert!(!code.contains("if is_hovering"));
        assert!(!code.contains("if !is_hovering"));
    }

    #[test]
    fn apply_hover_event_onmouseenter() {
        let handler = EventHandler::Ident("on_enter".into());
        let code = apply_hover_event("onmouseenter", &handler);
        assert!(code.starts_with(".on_hover("));
        assert!(code.contains("if is_hovering"));
        assert!(code.contains("this.on_enter"));
    }

    #[test]
    fn apply_hover_event_onmouseleave() {
        let handler = EventHandler::Ident("on_leave".into());
        let code = apply_hover_event("onmouseleave", &handler);
        assert!(code.starts_with(".on_hover("));
        assert!(code.contains("if !is_hovering"));
        assert!(code.contains("this.on_leave"));
    }

    #[test]
    fn apply_hover_event_via_apply_event_routing() {
        // apply_event 应委托给 apply_hover_event
        let handler = EventHandler::Ident("on_hover_change".into());
        let direct = apply_hover_event("onhover", &handler);
        let via_apply = apply_event("onhover", &handler, &ctx());
        assert_eq!(direct, via_apply);
    }

    #[test]
    fn apply_hover_event_method_name_handler() {
        let handler = EventHandler::MethodName("handle_hover".into());
        let code = apply_hover_event("onhover", &handler);
        assert!(code.contains("this.handle_hover"));
    }

    #[test]
    fn apply_hover_event_with_args_uses_method_only() {
        // WithArgs 的参数在 hover 事件中被忽略（仅取方法名）
        let handler = EventHandler::WithArgs("on_hover_change".into(), vec!["extra".into()]);
        let code = apply_hover_event("onhover", &handler);
        assert!(code.contains("this.on_hover_change"));
        assert!(!code.contains("p0"));
    }
}

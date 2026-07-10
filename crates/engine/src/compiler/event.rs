//! 事件绑定代码生成
//!
//! 将 RML 事件名（声明式 `on-click` kebab-case，normalize 后内部 `on_click` snake_case）
//! 映射到 GPUI 元素方法（如 `.on_click(...)`），并生成带 `cx.listener` 包装的闭包代码。
//!
//! ## 事件分类
//!
//! | 类型 | 声明式示例 | 内部 match | GPUI 回调签名 | 处理方式 |
//! |------|-----------|------------|---------------|---------|
//! | 标准事件 | on-click, on-key-down | on_click, on_key_down | `Fn(&EventType, &mut Window, &mut App)` | `apply_event` |
//! | 悬停事件 | on-hover, on-mouse-enter | on_hover, on_mouse_enter | `Fn(&bool, &mut Window, &mut App)` | `apply_hover_event` |
//! | 不支持 | on-input, on-change | on_input, on_change | GPUI 元素无对应方法 | 返回空字符串 |
//!
//! 详见文档 §5.4 事件绑定与 §10.6 代码生成。

use crate::compiler::{expr, CodegenCtx};
use crate::parser::ast::{Attribute, Element, EventHandler};

/// 检测当前是否在 slot 闭包上下文内（self_alias == "__rml_self_ref"）
///
/// slot 闭包内 cx 类型为 `&mut gpui::App`（非 `&mut Context<Self>`），
/// `cx.listener` 不可用，需改用 entity 捕获模式：
/// `__rml_self_entity.update(cx, |this, cx| { ... })`
pub(crate) fn in_slot_context() -> bool {
    expr::current_self_alias() == Some("__rml_self_ref")
}

/// 事件名 → (GPUI 事件类型, GPUI on_* 方法名, 转换函数路径)
///
/// 输入 `name` 为 normalize 后的 snake_case 形式（声明式 `on-click` → 内部 `on_click`）。
///
/// 返回 (event_type, on_method, convert_expr):
/// - `event_type`：GPUI 事件类型名，用于闭包参数标注
/// - `on_method`：GPUI StatefulInteractiveElement 方法名
/// - `convert_expr`：调用 rml_convert 的表达式，传入 GPUI 事件引用，返回 RML 事件对象
pub fn event_binding(name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match name {
        // 鼠标事件
        "on_click" => Some(("gpui::ClickEvent", "on_click", "rml_convert::from_gpui_click(ev)")),
        "on_aux_click" => Some(("gpui::ClickEvent", "on_aux_click", "rml_convert::from_gpui_click(ev)")),
        "on_mouse_down" => Some((
            "gpui::MouseDownEvent",
            "on_mouse_down",
            "rml_convert::from_gpui_mouse_down(ev)",
        )),
        "on_mouse_up" => Some((
            "gpui::MouseUpEvent",
            "on_mouse_up",
            "rml_convert::from_gpui_mouse_up(ev)",
        )),
        "on_mouse_move" => Some((
            "gpui::MouseMoveEvent",
            "on_mouse_move",
            "rml_convert::from_gpui_mouse_move(ev)",
        )),
        "on_wheel" => Some((
            "gpui::ScrollWheelEvent",
            "on_scroll_wheel",
            "rml_convert::from_gpui_scroll_wheel(ev)",
        )),
        // 键盘事件
        "on_key_down" => Some((
            "gpui::KeyDownEvent",
            "on_key_down",
            "rml_convert::from_gpui_key_down(ev)",
        )),
        "on_key_up" => Some((
            "gpui::KeyUpEvent",
            "on_key_up",
            "rml_convert::from_gpui_key_up(ev)",
        )),
        // 以下事件 GPUI 不直接提供对应类型，Phase B-2 由 codegen 直接构造 RML 事件
        // 暂不绑定（返回 None）
        "on_input" | "on_change" | "on_submit" | "on_load" | "on_resize" | "on_scroll" => None,
        _ => None,
    }
}

/// 判断事件是否为悬停类型（需要特殊处理 &bool 回调）
pub fn is_hover_event(name: &str) -> bool {
    matches!(name, "on_hover" | "on_mouse_enter" | "on_mouse_leave")
}

/// 判断事件是否为焦点类型（GPUI 回调无事件参数，3 参数闭包）
pub fn is_focus_event(name: &str) -> bool {
    matches!(name, "on_focus" | "on_blur")
}

/// P0-1：根据事件名返回对应的 handler 类型名（如 "ClickHandler"）
///
/// 用于 `gen_event_handler_assign` 生成闭包类型标注，确保类型推导正确。
/// 返回 None 表示事件不支持用户组件回调注入（如 hover 事件）。
pub fn handler_type_for_event(event_name: &str) -> Option<&'static str> {
    let gpui_type = event_binding(event_name)?.0;
    rml_core::event::handler_type_name(gpui_type)
}

/// 生成事件绑定代码
///
/// 标准事件：`.on_click(cx.listener(move |this, ev: &gpui::ClickEvent, _window, cx| { ... }))`
/// 悬停事件：委托给 `apply_hover_event`
/// on-action 事件：委托给 `apply_action_event`（多 Action 类型注册）
/// 不支持事件：返回空字符串
pub fn apply_event(name: &str, handler: &EventHandler, _ctx: &CodegenCtx) -> String {
    // on_hover 特殊处理：GPUI on_hover 回调接收 &bool 而非 &EventType
    if is_hover_event(name) {
        return apply_hover_event(name, handler);
    }

    // on_focus/on_blur 特殊处理：GPUI 回调签名为 Fn(&mut Window, &mut App)（无事件参数）
    if is_focus_event(name) {
        return apply_focus_event(name, handler);
    }

    // on_action 特殊处理：值为逗号分隔的 `ActionType:method` 对
    if name == "on_action" {
        return apply_action_event(handler);
    }

    // 未知事件名：跳过
    let (gpui_type, on_method, convert_expr) = match event_binding(name) {
        Some(binding) => binding,
        None => return String::new(),
    };

    let slot = in_slot_context();

    match handler {
        // P0-1：用户组件事件回调字段应用
        // 生成 .on_click(cx.listener(move |this, ev, _w, cx| {
        //     if let Some(h) = &this.<field> { h(ev, _w, &mut **cx); }
        // }))
        // `&mut **cx` 将 `&mut Context<Self>` 转换为 `&mut App`（Context<Self>: DerefMut<Target = App>）。
        EventHandler::ClosureField(field) => {
            if slot {
                format!(
                    ".{on_method}({{\n    \
                     let __rml_evt_entity = __rml_self_entity.clone();\n    \
                     move |ev: &{gpui_type}, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
                     __rml_evt_entity.update(cx, |this, cx| {{\n            \
                     if let Some(__rml_h) = &this.{field} {{\n                \
                     __rml_h(ev, _window, cx);\n            \
                     }}\n        \
                     }});\n    }}\n}})",
                    on_method = on_method,
                    gpui_type = gpui_type,
                    field = field,
                )
            } else {
                format!(
                    ".{on_method}(cx.listener(move |this, ev: &{gpui_type}, _window, cx| {{\n                    \
                     if let Some(__rml_h) = &this.{field} {{\n                        \
                     __rml_h(ev, _window, &mut **cx);\n                    \
                     }}\n                }}))",
                    on_method = on_method,
                    gpui_type = gpui_type,
                    field = field,
                )
            }
        }
        EventHandler::Ident(method) | EventHandler::MethodName(method) => {
            if slot {
                format!(
                    ".{on_method}({{\n    \
                     let __rml_evt_entity = __rml_self_entity.clone();\n    \
                     move |ev: &{gpui_type}, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
                     __rml_evt_entity.update(cx, |this, cx| {{\n            \
                     let rml_ev = {convert_expr};\n            \
                     this.{method}(&rml_ev, cx);\n            \
                     if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n        \
                     }});\n    }}\n}})",
                    on_method = on_method,
                    gpui_type = gpui_type,
                    convert_expr = convert_expr,
                    method = method,
                )
            } else {
                format!(
                    ".{}(cx.listener(move |this, ev: &{}, _window, cx| {{\n                    \
                     let rml_ev = {};\n                    this.{}(&rml_ev, cx);\n                    \
                     if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n                }}))",
                    on_method, gpui_type, convert_expr, method
                )
            }
        }
        EventHandler::WithArgs(method, args) => {
            if args.is_empty() {
                if slot {
                    format!(
                        ".{on_method}({{\n    \
                         let __rml_evt_entity = __rml_self_entity.clone();\n    \
                         move |ev: &{gpui_type}, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
                         __rml_evt_entity.update(cx, |this, cx| {{\n            \
                         let rml_ev = {convert_expr};\n            \
                         this.{method}(&rml_ev, cx);\n            \
                         if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n        \
                         }});\n    }}\n}})",
                        on_method = on_method,
                        gpui_type = gpui_type,
                        convert_expr = convert_expr,
                        method = method,
                    )
                } else {
                    format!(
                        ".{}(cx.listener(move |this, ev: &{}, _window, cx| {{\n                    \
                         let rml_ev = {};\n                    this.{}(&rml_ev, cx);\n                    \
                         if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n                }}))",
                        on_method, gpui_type, convert_expr, method
                    )
                }
            } else {
                // Phase B-1 简化：仅支持单参数
                let arg = &args[0];
                if slot {
                    format!(
                        ".{on_method}({{\n    \
                         let __rml_evt_entity = __rml_self_entity.clone();\n    \
                         move |ev: &{gpui_type}, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
                         __rml_evt_entity.update(cx, |this, cx| {{\n            \
                         let p0 = {arg}.clone();\n            \
                         let rml_ev = {convert_expr};\n            \
                         this.{method}(p0, &rml_ev, cx);\n            \
                         if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n        \
                         }});\n    }}\n}})",
                        on_method = on_method,
                        gpui_type = gpui_type,
                        arg = arg,
                        convert_expr = convert_expr,
                        method = method,
                    )
                } else {
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
}

/// 生成 on-action 事件绑定代码
///
/// `on-action` 属性值为逗号分隔的 `ActionType:method` 对，例如：
/// `"FormatDocument:on_format, RenameSymbol:on_rename"`
///
/// 对每对生成：
/// `.on_action::<ActionType>(cx.listener(move |this, _action: &ActionType, _window, cx| { this.method(_action, _window, cx); }))`
///
/// handler 方法签名：`fn method(&mut self, action: &ActionType, window: &mut Window, cx: &mut Context<Self>)`
///
/// 解析失败（空值、缺冒号、缺方法名）时返回空字符串（容错，不阻断编译）。
pub fn apply_action_event(handler: &EventHandler) -> String {
    let value = match handler {
        EventHandler::Ident(s) | EventHandler::MethodName(s) => s.as_str(),
        EventHandler::WithArgs(s, _) => s.as_str(),
        // P0-1：on-action 不支持闭包字段引用
        EventHandler::ClosureField(_) => return String::new(),
    };

    let slot = in_slot_context();
    let mut parts = Vec::new();
    for pair in value.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let Some((type_name, method)) = pair.split_once(':') else {
            return String::new();
        };
        let type_name = type_name.trim();
        let method = method.trim();
        if type_name.is_empty() || method.is_empty() {
            return String::new();
        }
        if slot {
            parts.push(format!(
                ".on_action::<{type_name}>({{\n    \
                 let __rml_evt_entity = __rml_self_entity.clone();\n    \
                 move |_action: &{type_name}, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
                 __rml_evt_entity.update(cx, |this, cx| {{\n            \
                 this.{method}(_action, _window, cx);\n        \
                 }});\n    }}\n}})"
            ));
        } else {
            parts.push(format!(
                ".on_action::<{type_name}>(cx.listener(move |this, _action: &{type_name}, _window, cx| {{\n                    \
                 this.{method}(_action, _window, cx);\n                }}))"
            ));
        }
    }
    parts.join(" ")
}

/// P0-1：生成 on-hover/on-mouse-enter/on-mouse-leave 闭包字段绑定代码
///
/// 与 `apply_hover_event` 同模式，但调用 `this.<field>` 闭包字段而非 command 方法。
/// on_mouse_enter/on_mouse_leave 应用条件过滤（仅 is_hovering == true / false 时触发）。
fn apply_hover_closure_field(name: &str, field: &str) -> String {
    let condition = match name {
        "on_mouse_enter" => Some("is_hovering"),
        "on_mouse_leave" => Some("!is_hovering"),
        _ => None,
    };

    let slot = in_slot_context();

    let body = if let Some(cond) = condition {
        if slot {
            format!(
                "if {} {{\n            \
                 if let Some(__rml_h) = &this.{} {{\n                \
                 __rml_h(is_hovering, _window, cx);\n            \
                 }}\n        \
                 }}",
                cond, field
            )
        } else {
            format!(
                "if {} {{\n                    \
                 if let Some(__rml_h) = &this.{} {{\n                        \
                 __rml_h(is_hovering, _window, &mut **cx);\n                    \
                 }}\n                \
                 }}",
                cond, field
            )
        }
    } else if slot {
        format!(
            "if let Some(__rml_h) = &this.{} {{\n            \
             __rml_h(is_hovering, _window, cx);\n        \
             }}",
            field
        )
    } else {
        format!(
            "if let Some(__rml_h) = &this.{} {{\n                    \
             __rml_h(is_hovering, _window, &mut **cx);\n                \
             }}",
            field
        )
    };

    if slot {
        format!(
            ".on_hover({{\n    \
             let __rml_evt_entity = __rml_self_entity.clone();\n    \
             move |is_hovering: &bool, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
             __rml_evt_entity.update(cx, |this, cx| {{\n            \
             {}\n        \
             }});\n    }}\n}})",
            body
        )
    } else {
        format!(
            ".on_hover(cx.listener(move |this, is_hovering: &bool, _window, cx| {{\n                    \
             {}\n                }}))",
            body
        )
    }
}

/// 生成 on-hover/on-mouse-enter/on-mouse-leave 事件绑定代码
///
/// GPUI `on_hover` 回调签名为 `Fn(&bool, &mut Window, &mut App)`，
/// `&bool` 为 true 表示进入，false 表示离开。
/// - `on-hover`：进入和离开都触发
/// - `on-mouse-enter`：仅 `is_hovering == true` 时触发
/// - `on-mouse-leave`：仅 `is_hovering == false` 时触发
pub fn apply_hover_event(name: &str, handler: &EventHandler) -> String {
    // P0-1：闭包字段引用（用户组件事件回调）
    if let EventHandler::ClosureField(field) = handler {
        return apply_hover_closure_field(name, field);
    }

    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => unreachable!(),
    };

    // on_mouse_enter/on_mouse_leave 需要条件过滤
    let condition = match name {
        "on_mouse_enter" => Some("is_hovering"),
        "on_mouse_leave" => Some("!is_hovering"),
        _ => None,
    };

    let slot = in_slot_context();

    let body = if let Some(cond) = condition {
        if slot {
            format!(
                "if {} {{\n            \
                 let rml_ev = rml_convert::from_gpui_hover(&is_hovering);\n            \
                 this.{}(&rml_ev, cx);\n        \
                 }}",
                cond, method
            )
        } else {
            format!(
                "if {} {{\n                    \
                 let rml_ev = rml_convert::from_gpui_hover(&is_hovering);\n                    \
                 this.{}(&rml_ev, cx);\n                }}",
                cond, method
            )
        }
    } else if slot {
        format!(
            "let rml_ev = rml_convert::from_gpui_hover(&is_hovering);\n            \
             this.{}(&rml_ev, cx);",
            method
        )
    } else {
        format!(
            "let rml_ev = rml_convert::from_gpui_hover(&is_hovering);\n                    \
             this.{}(&rml_ev, cx);",
            method
        )
    };

    if slot {
        format!(
            ".on_hover({{\n    \
             let __rml_evt_entity = __rml_self_entity.clone();\n    \
             move |is_hovering: &bool, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
             __rml_evt_entity.update(cx, |this, cx| {{\n            \
             {}\n        \
             }});\n    }}\n}})",
            body
        )
    } else {
        format!(
            ".on_hover(cx.listener(move |this, is_hovering: &bool, _window, cx| {{\n                    \
             {}\n                }}))",
            body
        )
    }
}

/// 生成 on-focus/on-blur 事件绑定代码
///
/// GPUI 的 `on_focus`/`on_blur` 是 `Context<T>` 级 API（非元素 builder 方法），
/// 不能用 `.on_focus(...)` 形式。焦点事件由 `gen_focus_event_setup` 生成预处理代码，
/// 元素链上用 `.track_focus(&handle)` 关联 FocusHandle。
///
/// 此函数返回空字符串，实际处理在 `meta.rs` 中通过 `gen_focus_event_setup` 完成。
pub fn apply_focus_event(_name: &str, _handler: &EventHandler) -> String {
    String::new()
}

/// 生成焦点事件预处理代码
///
/// GPUI 的 `on_focus`/`on_blur` 是 `Context<T>` 级 API，需要：
/// 1. 创建/获取 `FocusHandle`（缓存在 `RmlState.focus_handles`）
/// 2. 通过 `cx.on_focus(&handle, _window, listener).detach()` 注册监听器
/// 3. 元素链上用 `.track_focus(&handle)` 关联
///
/// 此函数生成步骤 1-2 的预处理代码，返回 `(预处理代码, handle 变量名)`。
/// 步骤 3 由 `meta.rs` 在元素链上添加 `.track_focus(&handle)`。
///
/// `dedup_key` 用作 `focus_handles` 和 `subscribed_events` 的键，格式如 `"focus_0"`。
pub fn gen_focus_event_setup(
    elem: &Element,
    dedup_key: &str,
) -> Option<(String, String)> {
    let on_focus_handler = elem.attributes.iter().find_map(|attr| {
        if let Attribute::Event { name, handler, .. } = attr {
            if name == "on_focus" { Some(handler) } else { None }
        } else {
            None
        }
    });
    let on_blur_handler = elem.attributes.iter().find_map(|attr| {
        if let Attribute::Event { name, handler, .. } = attr {
            if name == "on_blur" { Some(handler) } else { None }
        } else {
            None
        }
    });

    if on_focus_handler.is_none() && on_blur_handler.is_none() {
        return None;
    }

    let slot = in_slot_context();
    let self_prefix = if slot { "__rml_self_ref" } else { "self" };
    let handle_var = format!("__rml_focus_handle_{}", dedup_key);

    let mut pre = format!(
        "let {} = {}.__rml_state.get_or_init_focus_handle({:?}, cx);",
        handle_var, self_prefix, dedup_key
    );

    // 注册 on_focus 监听器
    if let Some(handler) = on_focus_handler {
        let subscribe_key = format!("{}:on_focus", dedup_key);
        let body = focus_listener_body(handler);
        pre.push_str(&format!(
            "\n    if !{}.__rml_state.is_event_subscribed({:?}) {{\n        \
             {}.__rml_state.mark_event_subscribed({:?}.to_string());\n        \
             cx.on_focus(&{}, _window, |this, _w, cx| {{\n            \
             {}\n        \
             }}).detach();\n    \
             }}",
            self_prefix, subscribe_key,
            self_prefix, subscribe_key,
            handle_var,
            body
        ));
    }

    // 注册 on_blur 监听器
    if let Some(handler) = on_blur_handler {
        let subscribe_key = format!("{}:on_blur", dedup_key);
        let body = focus_listener_body(handler);
        pre.push_str(&format!(
            "\n    if !{}.__rml_state.is_event_subscribed({:?}) {{\n        \
             {}.__rml_state.mark_event_subscribed({:?}.to_string());\n        \
             cx.on_blur(&{}, _window, |this, _w, cx| {{\n            \
             {}\n        \
             }}).detach();\n    \
             }}",
            self_prefix, subscribe_key,
            self_prefix, subscribe_key,
            handle_var,
            body
        ));
    }

    Some((pre, handle_var))
}

/// 生成焦点事件监听器闭包体
///
/// `cx.on_focus`/`cx.on_blur` 的 listener 签名为 `FnMut(&mut T, &mut Window, &mut Context<T>)`，
/// 闭包内 `this` 为 `&mut Self`，`cx` 为 `&mut Context<Self>`。
fn focus_listener_body(handler: &EventHandler) -> String {
    match handler {
        EventHandler::ClosureField(field) => {
            format!(
                "if let Some(__rml_h) = &this.{} {{\n                \
                 let rml_ev = rml_core::events::FocusEvent::default();\n                \
                 __rml_h(&rml_ev, _w, cx);\n            \
                 }}",
                field
            )
        }
        EventHandler::Ident(method) | EventHandler::MethodName(method) => {
            format!(
                "let rml_ev = rml_core::events::FocusEvent::default();\n                \
                 this.{}(&rml_ev, cx);",
                method
            )
        }
        EventHandler::WithArgs(method, args) => {
            if args.is_empty() {
                format!(
                    "let rml_ev = rml_core::events::FocusEvent::default();\n                \
                     this.{}(&rml_ev, cx);",
                    method
                )
            } else {
                // Phase B-1 简化：仅支持单参数
                let arg = &args[0];
                format!(
                    "let p0 = {}.clone();\n                \
                     let rml_ev = rml_core::events::FocusEvent::default();\n                \
                     this.{}(p0, &rml_ev, cx);",
                    arg, method
                )
            }
        }
    }
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
    fn event_binding_on_click() {
        let (ty, method, convert) = event_binding("on_click").unwrap();
        assert_eq!(ty, "gpui::ClickEvent");
        assert_eq!(method, "on_click");
        assert!(convert.contains("from_gpui_click"));
    }

    #[test]
    fn event_binding_on_aux_click() {
        let (_, method, _) = event_binding("on_aux_click").unwrap();
        assert_eq!(method, "on_aux_click");
    }

    #[test]
    fn event_binding_on_mouse_down() {
        let (ty, method, convert) = event_binding("on_mouse_down").unwrap();
        assert_eq!(ty, "gpui::MouseDownEvent");
        assert_eq!(method, "on_mouse_down");
        assert!(convert.contains("from_gpui_mouse_down"));
    }

    #[test]
    fn event_binding_on_mouse_up() {
        let (ty, method, _) = event_binding("on_mouse_up").unwrap();
        assert_eq!(ty, "gpui::MouseUpEvent");
        assert_eq!(method, "on_mouse_up");
    }

    #[test]
    fn event_binding_on_mouse_move() {
        let (ty, method, _) = event_binding("on_mouse_move").unwrap();
        assert_eq!(ty, "gpui::MouseMoveEvent");
        assert_eq!(method, "on_mouse_move");
    }

    #[test]
    fn event_binding_on_wheel() {
        let (ty, method, _) = event_binding("on_wheel").unwrap();
        assert_eq!(ty, "gpui::ScrollWheelEvent");
        assert_eq!(method, "on_scroll_wheel");
    }

    #[test]
    fn event_binding_on_key_down() {
        let (ty, method, _) = event_binding("on_key_down").unwrap();
        assert_eq!(ty, "gpui::KeyDownEvent");
        assert_eq!(method, "on_key_down");
    }

    #[test]
    fn event_binding_on_key_up() {
        let (ty, method, _) = event_binding("on_key_up").unwrap();
        assert_eq!(ty, "gpui::KeyUpEvent");
        assert_eq!(method, "on_key_up");
    }

    #[test]
    fn event_binding_unsupported_returns_none() {
        // GPUI 元素级不支持的事件
        assert!(event_binding("on_input").is_none());
        assert!(event_binding("on_change").is_none());
        assert!(event_binding("on_submit").is_none());
        assert!(event_binding("on_focus").is_none());
        assert!(event_binding("on_blur").is_none());
        assert!(event_binding("on_load").is_none());
        assert!(event_binding("on_resize").is_none());
        assert!(event_binding("on_scroll").is_none());
    }

    #[test]
    fn event_binding_unknown_returns_none() {
        assert!(event_binding("on_custom").is_none());
        assert!(event_binding("").is_none());
        assert!(event_binding("click").is_none()); // 缺少 on_ 前缀
    }

    #[test]
    fn event_binding_hover_returns_none() {
        // 悬停事件由 is_hover_event 单独处理，不在 event_binding 中
        assert!(event_binding("on_hover").is_none());
        assert!(event_binding("on_mouse_enter").is_none());
        assert!(event_binding("on_mouse_leave").is_none());
    }

    // ─── is_hover_event ───

    #[test]
    fn is_hover_event_recognizes_all_three() {
        assert!(is_hover_event("on_hover"));
        assert!(is_hover_event("on_mouse_enter"));
        assert!(is_hover_event("on_mouse_leave"));
    }

    #[test]
    fn is_hover_event_rejects_non_hover() {
        assert!(!is_hover_event("on_click"));
        assert!(!is_hover_event("on_mouse_down"));
        assert!(!is_hover_event(""));
        assert!(!is_hover_event("hover")); // 缺少 on_ 前缀
    }

    // ─── apply_event：标准事件 ───

    #[test]
    fn apply_event_on_click_ident() {
        let handler = EventHandler::Ident("increment".into());
        let code = apply_event("on_click", &handler, &ctx());
        assert!(code.starts_with(".on_click("));
        assert!(code.contains("gpui::ClickEvent"));
        assert!(code.contains("from_gpui_click(ev)"));
        assert!(code.contains("this.increment"));
        assert!(code.contains("cx.listener"));
    }

    #[test]
    fn apply_event_on_click_method_name() {
        let handler = EventHandler::MethodName("handle_click".into());
        let code = apply_event("on_click", &handler, &ctx());
        assert!(code.contains("this.handle_click"));
    }

    #[test]
    fn apply_event_on_key_down() {
        let handler = EventHandler::Ident("on_key".into());
        let code = apply_event("on_key_down", &handler, &ctx());
        assert!(code.starts_with(".on_key_down("));
        assert!(code.contains("gpui::KeyDownEvent"));
        assert!(code.contains("from_gpui_key_down(ev)"));
        assert!(code.contains("this.on_key"));
    }

    #[test]
    fn apply_event_with_args_empty() {
        // WithArgs 但参数为空，等同于无参数
        let handler = EventHandler::WithArgs("increment".into(), vec![]);
        let code = apply_event("on_click", &handler, &ctx());
        assert!(code.contains("this.increment"));
        assert!(!code.contains("p0"));
    }

    #[test]
    fn apply_event_with_args_single() {
        let handler = EventHandler::WithArgs("set_value".into(), vec!["42".into()]);
        let code = apply_event("on_click", &handler, &ctx());
        assert!(code.contains("let p0 = 42.clone();"));
        assert!(code.contains("this.set_value(p0,"));
    }

    #[test]
    fn apply_event_unsupported_returns_empty() {
        let handler = EventHandler::Ident("handler".into());
        assert_eq!(apply_event("on_input", &handler, &ctx()), "");
        assert_eq!(apply_event("on_custom", &handler, &ctx()), "");
    }

    // ─── 焦点事件（on_focus/on_blur）───
    // GPUI 的 on_focus/on_blur 是 Context 级 API，apply_focus_event 返回空字符串。
    // 实际处理由 gen_focus_event_setup 完成（预处理代码 + .track_focus）。

    #[test]
    fn apply_event_on_focus_returns_empty() {
        let handler = EventHandler::Ident("on_focus".into());
        let code = apply_event("on_focus", &handler, &ctx());
        assert_eq!(code, "");
    }

    #[test]
    fn apply_event_on_blur_returns_empty() {
        let handler = EventHandler::MethodName("handle_blur".into());
        let code = apply_event("on_blur", &handler, &ctx());
        assert_eq!(code, "");
    }

    #[test]
    fn is_focus_event_basic() {
        assert!(is_focus_event("on_focus"));
        assert!(is_focus_event("on_blur"));
        assert!(!is_focus_event("on_click"));
        assert!(!is_focus_event("on_hover"));
    }

    // ─── ClosureField（用户组件事件回调字段）───

    #[test]
    fn apply_event_closure_field_on_click() {
        let handler = EventHandler::ClosureField("on_click".into());
        let code = apply_event("on_click", &handler, &ctx());
        assert!(code.starts_with(".on_click("));
        assert!(code.contains("gpui::ClickEvent"));
        assert!(code.contains("if let Some(__rml_h) = &this.on_click"));
        assert!(code.contains("__rml_h(ev, _window, &mut **cx)"));
        assert!(code.contains("cx.listener"));
    }

    #[test]
    fn apply_event_closure_field_on_key_down() {
        let handler = EventHandler::ClosureField("on_key_down".into());
        let code = apply_event("on_key_down", &handler, &ctx());
        assert!(code.starts_with(".on_key_down("));
        assert!(code.contains("gpui::KeyDownEvent"));
        assert!(code.contains("if let Some(__rml_h) = &this.on_key_down"));
    }

    #[test]
    fn apply_event_closure_field_no_stop_propagation() {
        // ClosureField 不注入 stop_propagation 检查（由回调自身控制）
        let handler = EventHandler::ClosureField("on_click".into());
        let code = apply_event("on_click", &handler, &ctx());
        assert!(
            !code.contains("is_propagation_stopped"),
            "ClosureField 不应注入 stop_propagation 检查"
        );
    }

    // ─── stop_propagation 检查注入 ───

    #[test]
    fn stop_propagation_check_injected_ident() {
        // Ident handler 的闭包末尾应注入 stop_propagation 检查
        let handler = EventHandler::Ident("increment".into());
        let code = apply_event("on_click", &handler, &ctx());
        assert!(
            code.contains("if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }"),
            "Ident handler 应注入 stop_propagation 检查，实际：\n{}",
            code
        );
    }

    #[test]
    fn stop_propagation_check_injected_method_name() {
        let handler = EventHandler::MethodName("handle_click".into());
        let code = apply_event("on_click", &handler, &ctx());
        assert!(
            code.contains("if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }"),
            "MethodName handler 应注入 stop_propagation 检查"
        );
    }

    #[test]
    fn stop_propagation_check_injected_with_args_empty() {
        let handler = EventHandler::WithArgs("increment".into(), vec![]);
        let code = apply_event("on_click", &handler, &ctx());
        assert!(
            code.contains("if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }"),
            "WithArgs(空) handler 应注入 stop_propagation 检查"
        );
    }

    #[test]
    fn stop_propagation_check_injected_with_args_single() {
        let handler = EventHandler::WithArgs("set_value".into(), vec!["42".into()]);
        let code = apply_event("on_click", &handler, &ctx());
        assert!(
            code.contains("if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }"),
            "WithArgs(单参) handler 应注入 stop_propagation 检查"
        );
    }

    // ─── apply_hover_event ───

    #[test]
    fn apply_hover_event_on_hover() {
        let handler = EventHandler::Ident("on_hover_change".into());
        let code = apply_hover_event("on_hover", &handler);
        assert!(code.starts_with(".on_hover("));
        assert!(code.contains("is_hovering: &bool"));
        assert!(code.contains("from_gpui_hover"));
        assert!(code.contains("this.on_hover_change"));
        // on_hover 不应该有条件过滤
        assert!(!code.contains("if is_hovering"));
        assert!(!code.contains("if !is_hovering"));
    }

    #[test]
    fn apply_hover_event_on_mouse_enter() {
        let handler = EventHandler::Ident("on_enter".into());
        let code = apply_hover_event("on_mouse_enter", &handler);
        assert!(code.starts_with(".on_hover("));
        assert!(code.contains("if is_hovering"));
        assert!(code.contains("this.on_enter"));
    }

    #[test]
    fn apply_hover_event_on_mouse_leave() {
        let handler = EventHandler::Ident("on_leave".into());
        let code = apply_hover_event("on_mouse_leave", &handler);
        assert!(code.starts_with(".on_hover("));
        assert!(code.contains("if !is_hovering"));
        assert!(code.contains("this.on_leave"));
    }

    #[test]
    fn apply_hover_event_via_apply_event_routing() {
        // apply_event 应委托给 apply_hover_event
        let handler = EventHandler::Ident("on_hover_change".into());
        let direct = apply_hover_event("on_hover", &handler);
        let via_apply = apply_event("on_hover", &handler, &ctx());
        assert_eq!(direct, via_apply);
    }

    #[test]
    fn apply_hover_event_method_name_handler() {
        let handler = EventHandler::MethodName("handle_hover".into());
        let code = apply_hover_event("on_hover", &handler);
        assert!(code.contains("this.handle_hover"));
    }

    #[test]
    fn apply_hover_event_with_args_uses_method_only() {
        // WithArgs 的参数在 hover 事件中被忽略（仅取方法名）
        let handler = EventHandler::WithArgs("on_hover_change".into(), vec!["extra".into()]);
        let code = apply_hover_event("on_hover", &handler);
        assert!(code.contains("this.on_hover_change"));
        assert!(!code.contains("p0"));
    }

    // ─── apply_action_event ───

    #[test]
    fn apply_action_event_single_pair() {
        let handler = EventHandler::MethodName("FormatDocument:on_format".into());
        let code = apply_action_event(&handler);
        assert!(code.contains(".on_action::<FormatDocument>"));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("_action: &FormatDocument"));
        assert!(code.contains("this.on_format(_action, _window, cx)"));
    }

    #[test]
    fn apply_action_event_multiple_pairs() {
        let handler = EventHandler::MethodName(
            "FormatDocument:on_format, RenameSymbol:on_rename, FindReferences:on_refs".into(),
        );
        let code = apply_action_event(&handler);
        assert!(code.contains(".on_action::<FormatDocument>"));
        assert!(code.contains(".on_action::<RenameSymbol>"));
        assert!(code.contains(".on_action::<FindReferences>"));
        assert!(code.contains("this.on_format("));
        assert!(code.contains("this.on_rename("));
        assert!(code.contains("this.on_refs("));
    }

    #[test]
    fn apply_action_event_trims_whitespace() {
        let handler =
            EventHandler::MethodName("FormatDocument : on_format , RenameSymbol : on_rename".into());
        let code = apply_action_event(&handler);
        assert!(code.contains(".on_action::<FormatDocument>"));
        assert!(code.contains(".on_action::<RenameSymbol>"));
        assert!(code.contains("this.on_format("));
        assert!(code.contains("this.on_rename("));
    }

    #[test]
    fn apply_action_event_empty_returns_empty() {
        let handler = EventHandler::MethodName("".into());
        assert_eq!(apply_action_event(&handler), "");
    }

    #[test]
    fn apply_action_event_missing_colon_returns_empty() {
        let handler = EventHandler::MethodName("FormatDocument".into());
        assert_eq!(apply_action_event(&handler), "");
    }

    #[test]
    fn apply_action_event_empty_method_returns_empty() {
        let handler = EventHandler::MethodName("FormatDocument:".into());
        assert_eq!(apply_action_event(&handler), "");
    }

    #[test]
    fn apply_action_event_empty_type_returns_empty() {
        let handler = EventHandler::MethodName(":on_format".into());
        assert_eq!(apply_action_event(&handler), "");
    }

    #[test]
    fn apply_action_event_ident_handler() {
        let handler = EventHandler::Ident("FormatDocument:on_format".into());
        let code = apply_action_event(&handler);
        assert!(code.contains(".on_action::<FormatDocument>"));
        assert!(code.contains("this.on_format("));
    }

    #[test]
    fn apply_action_event_via_apply_event_routing() {
        let handler = EventHandler::MethodName("FormatDocument:on_format".into());
        let direct = apply_action_event(&handler);
        let via_apply = apply_event("on_action", &handler, &ctx());
        assert_eq!(direct, via_apply);
    }
}

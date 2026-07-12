//! 非 Input 状态事件订阅代码生成
//!
//! 与 `input/event.rs` 平行，处理拥有独立 Event 类型的 Stateful 组件
//! （ColorPickerEvent / CalendarEvent / DatePickerEvent 等）。
//!
//! ## 与 Input 事件的差异
//!
//! - Input 事件：`InputEvent::Change` 无载荷，用户方法接收 `&Entity<InputState>`
//! - State 事件：如 `ColorPickerEvent::Change(Option<Hsla>)` 带载荷，
//!   用户方法直接接收载荷值（如 `Option<Hsla>`），而非 Entity
//!
//! ## codegen 路径
//!
//! 1. `StatefulComponentTranslator` 在事件属性循环中调用 `is_state_event(name, tag)` 检测
//! 2. 若是 State 事件，跳过 `component_event_setter`，收集到 `state_event_handlers`
//! 3. `gen_stateful_body` 合并 Input 事件 + State 事件的 subscribe 代码到同一 block 表达式

use crate::parser::ast::EventHandler;
use crate::tags;

/// State 事件规格：描述如何订阅一个非 Input 的 Stateful 组件事件
#[derive(Debug, Clone, Copy)]
pub struct StateEventSpec {
    /// 组件的 canonical tag（如 "ColorPicker"）
    pub tag: &'static str,
    /// 事件属性名（如 "on_change"）
    pub event_name: &'static str,
    /// 事件类型路径（如 "rml_ui::ColorPickerEvent"）
    pub event_type: &'static str,
    /// 事件变体名（如 "Change"）
    pub event_variant: &'static str,
    /// 载荷绑定变量名（如 "color"），用于 `if let Event::Variant(payload) = event`
    pub payload_binding: &'static str,
    /// 用户方法调用模板（如 "this.{method}(color, cx)"），{method} 替换为方法名
    pub call_template: &'static str,
    /// 事件枚举是否仅有一个变体（可用 `let` 解构，避免 irrefutable_if_let 警告）
    pub irrefutable: bool,
}

/// State 事件注册表
pub static STATE_EVENT_REGISTRY: &[StateEventSpec] = &[
    StateEventSpec {
        tag: "ColorPicker",
        event_name: "on_change",
        event_type: "rml_ui::ColorPickerEvent",
        event_variant: "Change",
        payload_binding: "color",
        call_template: "this.{method}((*color).clone(), cx)",
        irrefutable: true,
    },
    StateEventSpec {
        tag: "Calendar",
        event_name: "on_select",
        event_type: "rml_ui::CalendarEvent",
        event_variant: "Selected",
        payload_binding: "date",
        call_template: "this.{method}((*date).clone(), cx)",
        irrefutable: true,
    },
    StateEventSpec {
        tag: "DatePicker",
        event_name: "on_change",
        event_type: "rml_ui::DatePickerEvent",
        event_variant: "Change",
        payload_binding: "date",
        call_template: "this.{method}((*date).clone(), cx)",
        irrefutable: true,
    },
    StateEventSpec {
        tag: "Select",
        event_name: "on_change",
        event_type: "rml_ui::SelectEvent",
        event_variant: "Confirm",
        payload_binding: "value",
        call_template: "this.{method}((*value).clone(), cx)",
        irrefutable: true,
    },
    StateEventSpec {
        tag: "Combobox",
        event_name: "on_change",
        event_type: "rml_ui::ComboboxEvent",
        event_variant: "Change",
        payload_binding: "values",
        call_template: "this.{method}((*values).clone(), cx)",
        irrefutable: false,
    },
    StateEventSpec {
        tag: "Slider",
        event_name: "on_change",
        event_type: "rml_ui::SliderEvent",
        event_variant: "Change",
        payload_binding: "value",
        call_template: "this.{method}((*value).clone(), cx)",
        irrefutable: false,
    },
];

/// 检测属性是否为 State 事件（非 Input 的 Stateful 组件事件）
pub fn is_state_event(name: &str, tag: &str) -> bool {
    let canonical = tags::canonical_tag(tag);
    STATE_EVENT_REGISTRY
        .iter()
        .any(|spec| spec.tag == canonical.as_str() && spec.event_name == name)
}

/// 查找 State 事件规格
pub fn lookup_state_event(name: &str, tag: &str) -> Option<&'static StateEventSpec> {
    let canonical = tags::canonical_tag(tag);
    STATE_EVENT_REGISTRY
        .iter()
        .find(|spec| spec.tag == canonical.as_str() && spec.event_name == name)
}

/// 生成 State 事件 subscribe 代码
///
/// 返回的字符串是一段独立语句（不带前导 `.`），用于嵌入到 block 表达式中。
pub fn gen_state_event_subscribe(
    ref_key: &str,
    event_name: &str,
    handler: &EventHandler,
    tag: &str,
) -> String {
    let spec = match lookup_state_event(event_name, tag) {
        Some(s) => s,
        None => return String::new(),
    };

    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };

    let call_expr = spec.call_template.replace("{method}", method);
    let event_pattern = if spec.irrefutable {
        format!(
            "let {}::{}({}) = event",
            spec.event_type, spec.event_variant, spec.payload_binding
        )
    } else {
        format!(
            "if let {}::{}({}) = event",
            spec.event_type, spec.event_variant, spec.payload_binding
        )
    };
    let event_body = if spec.irrefutable {
        format!("{event_pattern};\n                {call_expr};")
    } else {
        format!("{event_pattern} {{\n                    {call_expr};\n                }}")
    };
    let subscribe_key = format!("{}:{}", ref_key, event_name);

    let self_prefix = crate::compiler::expr::current_self_alias().unwrap_or("self");

    format!(
        "if !{self_prefix}.__rml_state.is_event_subscribed({subscribe_key:?}) {{\n            \
         cx.subscribe(&__rml_entity, |this, entity, event, cx| {{\n                \
         {event_body}\n            }}).detach();\n            \
         {self_prefix}.__rml_state.mark_event_subscribed({subscribe_key:?}.to_string());\n        \
         }}",
        self_prefix = self_prefix,
        subscribe_key = subscribe_key,
        event_body = event_body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_state_event_recognizes_color_picker() {
        assert!(is_state_event("on_change", "ColorPicker"));
        assert!(is_state_event("on_change", "color-picker"));
    }

    #[test]
    fn is_state_event_rejects_non_state_event() {
        assert!(!is_state_event("on_change", "Input"));
        assert!(!is_state_event("on_click", "ColorPicker"));
        assert!(!is_state_event("on_change", "Button"));
    }

    #[test]
    fn gen_subscribe_color_picker_change() {
        let handler = EventHandler::Ident("on_color_change".into());
        let code = gen_state_event_subscribe("color_picker_state", "on_change", &handler, "ColorPicker");
        assert!(code.contains("is_event_subscribed"), "code: {}", code);
        assert!(code.contains("\"color_picker_state:on_change\""), "code: {}", code);
        assert!(code.contains("cx.subscribe(&__rml_entity"), "code: {}", code);
        assert!(code.contains("rml_ui::ColorPickerEvent::Change(color)"), "code: {}", code);
        assert!(code.contains("this.on_color_change((*color).clone(), cx)"), "code: {}", code);
        assert!(code.contains("detach()"), "code: {}", code);
        assert!(code.contains("mark_event_subscribed"), "code: {}", code);
    }

    #[test]
    fn is_state_event_recognizes_date_picker() {
        assert!(is_state_event("on_change", "DatePicker"));
        assert!(is_state_event("on_change", "date-picker"));
    }

    #[test]
    fn gen_subscribe_date_picker_change() {
        let handler = EventHandler::Ident("on_date_change".into());
        let code = gen_state_event_subscribe("date_picker_state", "on_change", &handler, "DatePicker");
        assert!(code.contains("is_event_subscribed"), "code: {}", code);
        assert!(code.contains("\"date_picker_state:on_change\""), "code: {}", code);
        assert!(code.contains("cx.subscribe(&__rml_entity"), "code: {}", code);
        assert!(code.contains("rml_ui::DatePickerEvent::Change(date)"), "code: {}", code);
        assert!(code.contains("this.on_date_change((*date).clone(), cx)"), "code: {}", code);
        assert!(code.contains("detach()"), "code: {}", code);
        assert!(code.contains("mark_event_subscribed"), "code: {}", code);
    }

    #[test]
    fn is_state_event_recognizes_select() {
        assert!(is_state_event("on_change", "Select"));
        assert!(is_state_event("on_change", "select"));
    }

    #[test]
    fn gen_subscribe_select_change() {
        let handler = EventHandler::Ident("on_select_change".into());
        let code = gen_state_event_subscribe("select_state", "on_change", &handler, "Select");
        assert!(code.contains("is_event_subscribed"), "code: {}", code);
        assert!(code.contains("\"select_state:on_change\""), "code: {}", code);
        assert!(code.contains("cx.subscribe(&__rml_entity"), "code: {}", code);
        assert!(code.contains("rml_ui::SelectEvent::Confirm(value)"), "code: {}", code);
        assert!(code.contains("this.on_select_change((*value).clone(), cx)"), "code: {}", code);
        assert!(code.contains("detach()"), "code: {}", code);
        assert!(code.contains("mark_event_subscribed"), "code: {}", code);
    }

    #[test]
    fn is_state_event_recognizes_combobox() {
        assert!(is_state_event("on_change", "Combobox"));
        assert!(is_state_event("on_change", "combobox"));
    }

    #[test]
    fn gen_subscribe_combobox_change() {
        let handler = EventHandler::Ident("on_combobox_change".into());
        let code = gen_state_event_subscribe("combobox_state", "on_change", &handler, "Combobox");
        assert!(code.contains("is_event_subscribed"), "code: {}", code);
        assert!(code.contains("\"combobox_state:on_change\""), "code: {}", code);
        assert!(code.contains("cx.subscribe(&__rml_entity"), "code: {}", code);
        assert!(code.contains("rml_ui::ComboboxEvent::Change(values)"), "code: {}", code);
        assert!(code.contains("this.on_combobox_change((*values).clone(), cx)"), "code: {}", code);
        assert!(code.contains("detach()"), "code: {}", code);
        assert!(code.contains("mark_event_subscribed"), "code: {}", code);
    }

    #[test]
    fn is_state_event_recognizes_slider() {
        assert!(is_state_event("on_change", "Slider"));
    }

    #[test]
    fn gen_subscribe_slider_change() {
        let handler = EventHandler::Ident("on_slider_change".into());
        let code = gen_state_event_subscribe("slider_state", "on_change", &handler, "Slider");
        assert!(code.contains("is_event_subscribed"), "code: {}", code);
        assert!(code.contains("\"slider_state:on_change\""), "code: {}", code);
        assert!(code.contains("cx.subscribe(&__rml_entity"), "code: {}", code);
        assert!(code.contains("rml_ui::SliderEvent::Change(value)"), "code: {}", code);
        assert!(code.contains("this.on_slider_change((*value).clone(), cx)"), "code: {}", code);
        assert!(code.contains("detach()"), "code: {}", code);
        assert!(code.contains("mark_event_subscribed"), "code: {}", code);
    }

    #[test]
    fn gen_subscribe_unknown_returns_empty() {
        let handler = EventHandler::Ident("h".into());
        let code = gen_state_event_subscribe("state", "on_change", &handler, "Button");
        assert!(code.is_empty());
    }
}

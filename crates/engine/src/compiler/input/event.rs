//! Input / TextInput / CodeEditor 事件订阅代码生成
//!
//! Input element 没有 `.on_change()` / `.on_enter()` 等方法（gpui-component 设计），
//! 事件通过 `InputState: EventEmitter<InputEvent>` 发送，用户通过 `cx.subscribe` 订阅。
//!
//! ## codegen 路径
//!
//! 1. `gen_component` 在事件属性循环中调用 `is_input_event(name, tag)` 检测
//! 2. 若是 Input 事件，跳过 `component_event_setter`，收集到 `input_event_handlers`
//! 3. 在构造器生成完成后，若有 Input 事件，把构造器包装到 block 表达式中：
//!    ```ignore
//!    ({
//!        let __rml_entity = <ref 或 no-ref 路径生成的 Entity<InputState>>;
//!        if !self.__rml_state.is_event_subscribed("<ref>:on_change") {
//!            cx.subscribe(&__rml_entity, |this, entity, event, cx| {
//!                if let rml_ui::InputEvent::Change = event {
//!                    this.on_change(entity.read(cx), cx);
//!                }
//!            }).detach();
//!            self.__rml_state.mark_event_subscribed("<ref>:on_change".to_string());
//!        }
//!        rml_ui::Input::new(&__rml_entity)
//!    })
//!    ```
//!
//! 4. 后续 setter 链式调用此 block 表达式（`.disabled(true)` 等）
//!
//! ## 设计说明
//!
//! - subscription 句柄用 `detach()` 让其随 entity 生命周期自动销毁，无需手动管理
//! - 用 `RmlState::is_event_subscribed` 防止重复 subscribe（每次 render 都会重新评估）
//! - 闭包接收 `&InputEvent`，按 variant 分发到用户方法

use crate::parser::ast::EventHandler;
use crate::tags;

/// 检测属性是否为 Input 事件（仅 Input / TextInput / CodeEditor 支持）
pub fn is_input_event(name: &str, tag: &str) -> bool {
    let canonical = tags::canonical_tag(tag);
    if canonical != "Input" && canonical != "TextInput" && canonical != "CodeEditor" {
        return false;
    }
    matches!(name, "on_change" | "on_enter" | "on_focus" | "on_blur")
}

/// 生成 Input 事件 subscribe 代码
///
/// 返回的字符串是一段独立语句（不带前导 `.`），用于嵌入到 block 表达式中。
///
/// 参数:
/// - `ref_key`: subscribe 标识键，通常为 `ref="name"` 的 name 或 state_field 名
/// - `event_name`: `on_change` / `on_enter` / `on_focus` / `on_blur`
/// - `handler`: 用户方法标识（Ident / MethodName / WithArgs）
pub fn gen_input_event_subscribe(
    ref_key: &str,
    event_name: &str,
    handler: &EventHandler,
) -> String {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
    };

    let (event_pattern, call_expr) = match event_name {
        "on_change" => (
            "if let rml_ui::InputEvent::Change = event",
            "this.{method}(entity.read(cx), cx)",
        ),
        "on_enter" => (
            "if let rml_ui::InputEvent::PressEnter { .. } = event",
            "this.{method}(entity.read(cx), cx)",
        ),
        "on_focus" => (
            "if let rml_ui::InputEvent::Focus = event",
            "this.{method}(entity.read(cx), cx)",
        ),
        "on_blur" => (
            "if let rml_ui::InputEvent::Blur = event",
            "this.{method}(entity.read(cx), cx)",
        ),
        _ => return String::new(),
    };

    let call_expr = call_expr.replace("{method}", method);
    let subscribe_key = format!("{}:{}", ref_key, event_name);

    format!(
        "if !self.__rml_state.is_event_subscribed({subscribe_key:?}) {{\n            \
         cx.subscribe(&__rml_entity, |this, entity, event, cx| {{\n                \
         {event_pattern} {{\n                    \
         {call_expr};\n                \
         }}\n            }}).detach();\n            \
         self.__rml_state.mark_event_subscribed({subscribe_key:?}.to_string());\n        \
         }}",
        subscribe_key = subscribe_key,
        event_pattern = event_pattern,
        call_expr = call_expr,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_input_event_recognizes_input_events() {
        assert!(is_input_event("on_change", "Input"));
        assert!(is_input_event("on_enter", "Input"));
        assert!(is_input_event("on_focus", "TextInput"));
        assert!(is_input_event("on_blur", "CodeEditor"));
    }

    #[test]
    fn is_input_event_rejects_non_input_tag() {
        assert!(!is_input_event("on_change", "Button"));
        assert!(!is_input_event("on_change", "Slider"));
    }

    #[test]
    fn is_input_event_rejects_non_input_event() {
        assert!(!is_input_event("on_click", "Input"));
        assert!(!is_input_event("on_hover", "Input"));
    }

    #[test]
    fn gen_subscribe_on_change_input() {
        let handler = EventHandler::Ident("on_input_change".into());
        let code = gen_input_event_subscribe("input_state", "on_change", &handler);
        assert!(code.contains("is_event_subscribed"), "code: {}", code);
        assert!(code.contains("\"input_state:on_change\""), "code: {}", code);
        assert!(code.contains("cx.subscribe(&__rml_entity"), "code: {}", code);
        assert!(code.contains("InputEvent::Change"), "code: {}", code);
        assert!(code.contains("this.on_input_change(entity.read(cx), cx)"), "code: {}", code);
        assert!(code.contains("detach()"), "code: {}", code);
        assert!(code.contains("mark_event_subscribed"), "code: {}", code);
    }

    #[test]
    fn gen_subscribe_on_enter_textinput() {
        let handler = EventHandler::MethodName("handle_enter".into());
        let code = gen_input_event_subscribe("text_state", "on_enter", &handler);
        assert!(code.contains("\"text_state:on_enter\""), "code: {}", code);
        assert!(code.contains("InputEvent::PressEnter"), "code: {}", code);
        assert!(code.contains("this.handle_enter(entity.read(cx), cx)"), "code: {}", code);
    }

    #[test]
    fn gen_subscribe_on_focus_code_editor() {
        let handler = EventHandler::Ident("on_focus_handler".into());
        let code = gen_input_event_subscribe("editor_state", "on_focus", &handler);
        assert!(code.contains("\"editor_state:on_focus\""), "code: {}", code);
        assert!(code.contains("InputEvent::Focus"), "code: {}", code);
        assert!(code.contains("this.on_focus_handler"), "code: {}", code);
    }

    #[test]
    fn gen_subscribe_on_blur_input() {
        let handler = EventHandler::Ident("on_blur_handler".into());
        let code = gen_input_event_subscribe("input_state", "on_blur", &handler);
        assert!(code.contains("\"input_state:on_blur\""), "code: {}", code);
        assert!(code.contains("InputEvent::Blur"), "code: {}", code);
        assert!(code.contains("this.on_blur_handler"), "code: {}", code);
    }
}

//! Input / TextInput / CodeEditor 事件 setter —— `on_change` 回调生成。
//!
//! 声明式 `on-change={fn}`（kebab-case），normalize 后内部 match `on_change`（snake_case）。
//! 仅 Input / TextInput / CodeEditor 组件支持（CodeEditor 基于 Input），其他组件调用时
//! 返回 None（由 component_event_setter 回退处理）。

use crate::parser::ast::EventHandler;
use crate::tags;

/// Input / TextInput / CodeEditor 专用事件 setter
///
/// `on-change={fn}` → `.on_change(cx.listener(move |this, state: &rml_ui::InputState, _window, cx| { this.fn(state, cx); }))`
///
/// 用户方法签名约定：`fn on_change(&mut self, state: &rml_ui::InputState, cx: &mut Context<Self>)`
///
/// 仅当 `canonical_tag(tag)` 为 `Input` / `TextInput` / `CodeEditor` 时匹配，否则返回 None。
pub fn event_setter(name: &str, handler: &EventHandler, tag: &str) -> Option<String> {
    let canonical = tags::canonical_tag(tag);
    if canonical != "Input" && canonical != "TextInput" && canonical != "CodeEditor" {
        return None;
    }

    match name {
        "on_change" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_change(cx.listener(move |this, state: &rml_ui::InputState, _window, cx| {{\n                    \
                 this.{}(state, cx);\n                }}))",
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
    fn event_setter_on_change_input() {
        let handler = EventHandler::Ident("on_change".into());
        let code = event_setter("on_change", &handler, "Input").unwrap();
        assert!(code.starts_with(".on_change("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("state: &rml_ui::InputState"));
        assert!(code.contains("this.on_change"));
    }

    #[test]
    fn event_setter_on_change_textinput() {
        let handler = EventHandler::MethodName("handle_change".into());
        let code = event_setter("on_change", &handler, "TextInput").unwrap();
        assert!(code.contains("this.handle_change"));
    }

    #[test]
    fn event_setter_on_change_code_editor() {
        let handler = EventHandler::Ident("on_editor_change".into());
        let code = event_setter("on_change", &handler, "CodeEditor").unwrap();
        assert!(code.starts_with(".on_change("));
        assert!(code.contains("this.on_editor_change"));
    }

    #[test]
    fn event_setter_returns_none_for_non_input_tag() {
        let handler = EventHandler::Ident("on_change".into());
        assert!(event_setter("on_change", &handler, "Button").is_none());
    }

    #[test]
    fn event_setter_returns_none_for_unknown_event() {
        let handler = EventHandler::Ident("on_click".into());
        assert!(event_setter("on_click", &handler, "Input").is_none());
    }
}

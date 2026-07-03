//! Input / TextInput 事件 setter —— `on_change` 回调生成。

use crate::parser::ast::EventHandler;

/// Input / TextInput 专用事件 setter
///
/// `onchange={fn}` → `.on_change(cx.listener(move |this, state: &rml_ui::InputState, _window, cx| { this.fn(state, cx); }))`
///
/// 用户方法签名约定：`fn on_change(&mut self, state: &rml_ui::InputState, cx: &mut Context<Self>)`
pub fn event_setter(name: &str, handler: &EventHandler, _tag: &str) -> Option<String> {
    match name {
        "onchange" => {
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
    fn event_setter_onchange_input() {
        let handler = EventHandler::Ident("on_change".into());
        let code = event_setter("onchange", &handler, "Input").unwrap();
        assert!(code.starts_with(".on_change("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("state: &rml_ui::InputState"));
        assert!(code.contains("this.on_change"));
    }

    #[test]
    fn event_setter_onchange_textinput() {
        let handler = EventHandler::MethodName("handle_change".into());
        let code = event_setter("onchange", &handler, "TextInput").unwrap();
        assert!(code.contains("this.handle_change"));
    }

    #[test]
    fn event_setter_returns_none_for_unknown() {
        let handler = EventHandler::Ident("on_click".into());
        assert!(event_setter("onclick", &handler, "Input").is_none());
    }
}

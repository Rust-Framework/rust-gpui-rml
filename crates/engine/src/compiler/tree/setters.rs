//! Tree 专用事件 setter —— `on_activate` 回调生成。

use crate::parser::ast::EventHandler;

/// Tree 专用事件 setter
///
/// `on_activate={fn}` → `.on_activate_rc(Rc::new({ let weak = cx.weak_entity(); move |item, ...| {...} }))`
///
/// 用户方法签名约定：`fn on_activate(&mut self, item_id: &str, cx: &mut Context<Self>)`
pub fn event_setter(name: &str, handler: &EventHandler, _tag: &str) -> Option<String> {
    match name {
        "on_activate" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_activate_rc(std::rc::Rc::new({{\n                    \
                 let weak = cx.weak_entity();\n                    \
                 move |item: rml_ui::TreeItem, _window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
                 if let Some(entity) = weak.upgrade() {{\n                            \
                 entity.update(app, |this, cx| {{ this.{}(&item.id, cx); }});\n                        \
                 }}\n                    }}\n                }}))",
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
    fn event_setter_on_activate() {
        let handler = EventHandler::Ident("on_activate".into());
        let code = event_setter("on_activate", &handler, "Tree").unwrap();
        assert!(code.starts_with(".on_activate_rc("));
        assert!(code.contains("cx.weak_entity()"));
        assert!(code.contains("this.on_activate"));
    }

    #[test]
    fn event_setter_returns_none_for_unknown() {
        let handler = EventHandler::Ident("on_click".into());
        assert!(event_setter("onclick", &handler, "Tree").is_none());
    }
}

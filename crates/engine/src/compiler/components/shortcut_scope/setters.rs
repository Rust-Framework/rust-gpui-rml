//! ShortcutScope 专用属性 setter（Shortcut 元数据 → `.shortcut(key, when, handler)`）

use crate::parser::ast::{Attribute, Element, EventHandler};

/// 从 `<Shortcut>` 元素生成 `.shortcut(...)` 链式调用片段
pub fn gen_shortcut_call(elem: &Element) -> Result<String, crate::compiler::CodegenError> {
    let mut key: Option<&str> = None;
    let mut when_expr: Option<String> = None;
    let mut on_press: Option<String> = None;

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == "key" => {
                key = Some(value.as_str());
            }
            Attribute::Bind { name, expr, .. } if name == "when" => {
                when_expr = Some(crate::compiler::setters::component_bind_rust_expr(expr, &[], &[]));
            }
            Attribute::Event { name, handler, .. } if name == "on_press" => {
                on_press = Some(gen_on_press_handler(handler));
            }
            _ => {}
        }
    }

    let key = key.ok_or_else(|| crate::compiler::CodegenError {
        message: "<Shortcut> 必须指定 key 属性".into(),
        span: Some(elem.span),
    })?;
    let when = when_expr.unwrap_or_else(|| "true".to_string());
    let on_press = on_press.ok_or_else(|| crate::compiler::CodegenError {
        message: "<Shortcut> 必须指定 on-press 属性".into(),
        span: Some(elem.span),
    })?;

    Ok(format!(
        ".shortcut({key:?}, {when}, {on_press})",
        key = key,
        when = when,
        on_press = on_press
    ))
}

fn gen_on_press_handler(handler: &EventHandler) -> String {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };
    format!(
        "{{\n                    \
         let __entity = cx.entity();\n                    \
         move |_window, cx| {{\n                        \
         __entity.update(cx, |this, cx| {{ this.{method}(cx); }});\n                    \
         }}\n                }}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Attribute, Element, EventHandler};
    use crate::parser::Span;

    fn shortcut_elem(key: &str, handler: &str) -> Element {
        Element {
            tag: "Shortcut".into(),
            attributes: vec![
                Attribute::Static {
                    name: "key".into(),
                    value: key.into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_press".into(),
                    handler: EventHandler::Ident(handler.into()),
                    span: Span::empty(),
                },
            ],
            directives: vec![],
            children: vec![],
            slot_name: None,
            span: Span::empty(),
        }
    }

    #[test]
    fn gen_shortcut_call_basic() {
        let elem = shortcut_elem("Ctrl+S", "on_save");
        let code = gen_shortcut_call(&elem).unwrap();
        assert!(code.contains(".shortcut(\"Ctrl+S\", true,"));
        assert!(code.contains("this.on_save(cx)"));
    }

    #[test]
    fn gen_shortcut_call_with_when() {
        let mut elem = shortcut_elem("Ctrl+D", "on_debug");
        elem.attributes.push(Attribute::Bind {
            name: "when".into(),
            expr: "shortcut_enabled".into(),
            span: Span::empty(),
        });
        let code = gen_shortcut_call(&elem).unwrap();
        assert!(code.contains(".shortcut(\"Ctrl+D\", self.shortcut_enabled,"));
    }

    #[test]
    fn gen_shortcut_call_rejects_missing_key() {
        let elem = Element {
            tag: "Shortcut".into(),
            attributes: vec![Attribute::Event {
                name: "on_press".into(),
                handler: EventHandler::Ident("on_save".into()),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: None,
            span: Span::empty(),
        };
        let err = gen_shortcut_call(&elem).unwrap_err();
        assert!(err.message.contains("key"));
    }

    #[test]
    fn gen_shortcut_call_rejects_missing_on_press() {
        let elem = Element {
            tag: "Shortcut".into(),
            attributes: vec![Attribute::Static {
                name: "key".into(),
                value: "Ctrl+S".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: None,
            span: Span::empty(),
        };
        let err = gen_shortcut_call(&elem).unwrap_err();
        assert!(err.message.contains("on-press"));
    }
}

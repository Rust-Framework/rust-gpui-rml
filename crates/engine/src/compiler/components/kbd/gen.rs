//! Kbd 构造代码生成
//!
//! ## 属性映射
//!
//! - `key="cmd-a"` → `Kbd::new(gpui::Keystroke::parse("cmd-a").expect("valid keystroke"))`
//! - `outline=""` → `.outline()`
//! - `appearance="false"` → `.appearance(false)`
//! - `size` 等通用属性走 `component_static_setter` 链

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element};
use crate::tags;

use super::setters::kbd_static_setter;

/// 生成 Kbd 构造代码
pub fn gen_kbd(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let resolved = "Kbd";
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. 构造器：从 key 属性提取 Keystroke
    let mut code = String::new();
    let mut key_set = false;

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == "key" => {
                // key="cmd-a" → Kbd::new(gpui::Keystroke::parse("cmd-a").expect("valid keystroke"))
                code.push_str(&format!(
                    "rml_ui::Kbd::new(gpui::Keystroke::parse({:?}).expect(\"valid keystroke\"))",
                    value
                ));
                key_set = true;
            }
            Attribute::Bind { name, expr, .. } if name == "key" => {
                // key={keystroke_expr} → Kbd::new(keystroke_expr)
                // 注：绑定表达式需返回 Keystroke 类型
                let rust_expr =
                    crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed);
                code.push_str(&format!("rml_ui::Kbd::new({})", rust_expr));
                key_set = true;
            }
            _ => {}
        }
    }

    if !key_set {
        return Err(CodegenError {
            message: "<Kbd> requires `key=\"...\"` attribute (e.g. <Kbd key=\"cmd-a\" />)".into(),
            span: Some(elem.span),
        });
    }

    // 2. Kbd 专用属性 → builder 方法
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if name == "key" {
                    continue;
                }
                // Kbd 专用属性
                if let Some(s) = kbd_static_setter(name, value) {
                    code.push_str(&s);
                    continue;
                }
                // 通用属性
                if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, resolved)
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if name == "key" {
                    continue;
                }
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, resolved,
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, resolved)
                {
                    code.push_str(&s);
                }
            }
        }
    }

    let _ = id_counter;
    let _ = tags::canonical_tag(&elem.tag);
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Element, Node};
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    fn make_element(tag: &str, attrs: Vec<Attribute>, children: Vec<Node>) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives: vec![],
            children,
            slot_name: None,
            ..Default::default()
        }
    }

    #[test]
    fn gen_kbd_basic() {
        // <Kbd key="cmd-a" />
        let elem = make_element(
            "Kbd",
            vec![Attribute::Static {
                name: "key".into(),
                value: "cmd-a".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_kbd(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Kbd::new(gpui::Keystroke::parse(\"cmd-a\").expect(\"valid keystroke\"))"));
    }

    #[test]
    fn gen_kbd_with_outline() {
        // <Kbd key="ctrl-a" outline="" />
        let elem = make_element(
            "Kbd",
            vec![
                Attribute::Static {
                    name: "key".into(),
                    value: "ctrl-a".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "outline".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_kbd(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("Kbd::new(gpui::Keystroke::parse(\"ctrl-a\")"));
        assert!(code.contains(".outline()"));
    }

    #[test]
    fn gen_kbd_appearance_false() {
        // <Kbd key="shift-space" appearance="false" />
        let elem = make_element(
            "Kbd",
            vec![
                Attribute::Static {
                    name: "key".into(),
                    value: "shift-space".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "appearance".into(),
                    value: "false".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_kbd(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains(".appearance(false)"));
    }

    #[test]
    fn gen_kbd_appearance_true_no_op() {
        // <Kbd key="a" appearance="true" /> → appearance 默认值，无操作
        let elem = make_element(
            "Kbd",
            vec![
                Attribute::Static {
                    name: "key".into(),
                    value: "a".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "appearance".into(),
                    value: "true".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_kbd(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        // appearance=true 是默认值，不应生成 .appearance(true)
        assert!(!code.contains(".appearance"));
    }

    #[test]
    fn gen_kbd_missing_key_returns_error() {
        // <Kbd /> → 缺少 key 属性，返回错误
        let elem = make_element("Kbd", vec![], vec![]);
        let result = gen_kbd(&elem, &ctx(), &mut 0, &Vec::new());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("requires `key"));
    }

    #[test]
    fn gen_kbd_key_bind() {
        // <Kbd key={keystroke} /> → Kbd::new(self.keystroke)
        let elem = make_element(
            "Kbd",
            vec![Attribute::Bind {
                name: "key".into(),
                expr: "keystroke".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_kbd(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Kbd::new(self.keystroke)"));
    }
}

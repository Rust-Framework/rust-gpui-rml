//! HoverCard 构造代码生成
//!
//! ## 子节点处理
//!
//! - `slot="trigger"` 的子元素 → `.trigger(element)`
//! - 其余子元素 → `.child(element)`（content）

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters::static_setter;

/// 生成 HoverCard 构造代码
pub fn gen_hover_card(
    elem: &Element,
    ref_name: Option<&str>,
    id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器
    let mut code = if let Some(name) = ref_name {
        format!("rml_ui::HoverCard::new({:?})", format!("rml_ref:{}", name))
    } else {
        format!("rml_ui::HoverCard::new((\"rml_el\", {}usize))", id_val)
    };

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "HoverCard", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "HoverCard")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "HoverCard",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "HoverCard")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：slot="trigger" → .trigger()，其余 → .child()
    let mut trigger_code: Option<String> = None;
    let mut content_codes: Vec<String> = Vec::new();

    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        match child {
            crate::parser::ast::Node::Element(e) if e.slot_name.as_deref() == Some("trigger") => {
                if is_iter {
                    return Err(CodegenError {
                        message: "HoverCard trigger slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                if trigger_code.is_some() {
                    return Err(CodegenError {
                        message: "HoverCard requires exactly one trigger slot (multiple found)".into(),
                        span: Some(elem.span),
                    });
                }
                trigger_code = Some(child_code);
            }
            _ => {
                if is_iter {
                    content_codes.push(format!(".children({})", child_code));
                } else {
                    content_codes.push(format!(".child({})", child_code));
                }
            }
        }
    }

    // 先注入 trigger，再注入 content
    if let Some(tc) = trigger_code {
        code.push_str(&format!("\n            .trigger({})", tc));
    }
    for content_code in content_codes {
        code.push_str(&format!("\n            {}", content_code));
    }

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

    fn make_trigger() -> Element {
        Element {
            tag: "Button".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: "Hover me".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        }
    }

    #[test]
    fn gen_hover_card_minimal() {
        let elem = make_element("HoverCard", vec![], vec![Node::Element(make_trigger())]);
        let code = gen_hover_card(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::HoverCard::new((\"rml_el\", 0usize))"));
        assert!(code.contains(".trigger("));
        assert!(code.contains("Button::new"));
    }

    #[test]
    fn gen_hover_card_with_content() {
        let content = Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("Hello".into())],
            slot_name: None,
            ..Default::default()
        };
        let elem = make_element(
            "HoverCard",
            vec![],
            vec![Node::Element(make_trigger()), Node::Element(content)],
        );
        let code = gen_hover_card(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".trigger("));
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_hover_card_with_anchor() {
        let elem = make_element(
            "HoverCard",
            vec![Attribute::Static {
                name: "anchor".into(),
                value: "bottom-right".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(make_trigger())],
        );
        let code = gen_hover_card(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".anchor(gpui::Anchor::BottomRight)"));
    }

    #[test]
    fn gen_hover_card_with_delays() {
        let elem = make_element(
            "HoverCard",
            vec![
                Attribute::Static {
                    name: "open_delay".into(),
                    value: "800".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "close_delay".into(),
                    value: "400".into(),
                    span: Span::empty(),
                },
            ],
            vec![Node::Element(make_trigger())],
        );
        let code = gen_hover_card(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".open_delay(std::time::Duration::from_millis(800))"));
        assert!(code.contains(".close_delay(std::time::Duration::from_millis(400))"));
    }

    #[test]
    fn gen_hover_card_no_trigger() {
        let content = Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        };
        let elem = make_element("HoverCard", vec![], vec![Node::Element(content)]);
        let code = gen_hover_card(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(!code.contains(".trigger("));
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_hover_card_multiple_triggers_error() {
        let elem = make_element(
            "HoverCard",
            vec![],
            vec![Node::Element(make_trigger()), Node::Element(make_trigger())],
        );
        let result = gen_hover_card(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("exactly one trigger"));
    }

    #[test]
    fn gen_hover_card_with_ref() {
        let elem = make_element("HoverCard", vec![], vec![Node::Element(make_trigger())]);
        let code = gen_hover_card(&elem, Some("my_card"), 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::HoverCard::new(\"rml_ref:my_card\")"));
    }
}

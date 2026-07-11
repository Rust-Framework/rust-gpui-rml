//! AlertDialog 构造代码生成
//!
//! ## 构造器
//!
//! `AlertDialog::new(cx: &mut App)` —— 直接使用 render 上下文的 `cx` 变量。
//!
//! ## 子节点处理
//!
//! - `slot="trigger"` 的子元素 → `.trigger(element)`（同 Dialog/HoverCard）
//! - `slot="footer"` 的子元素 → `.footer(element)`（自定义页脚，同 Dialog）
//! - 其余子元素 → `.child(element)` / `.children(iterator)`（ParentElement）

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters::{event_setter, static_setter};

/// 生成 AlertDialog 构造代码
pub fn gen_alert_dialog(
    elem: &Element,
    _ref_name: Option<&str>,
    _id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：AlertDialog::new(cx)
    let mut code = "rml_ui::AlertDialog::new(cx)".to_string();

    // CSS class 样式
    append_css_class_styles(
        &mut code,
        elem,
        "AlertDialog",
        ctx.stylesheet.as_ref(),
        parents,
    );

    // 2. 属性 → setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "AlertDialog")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "AlertDialog",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = event_setter(name, handler) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "AlertDialog")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：slot="trigger" → .trigger()，slot="footer" → .footer()，其余 → .child() / .children()
    let mut trigger_code: Option<String> = None;
    let mut footer_code: Option<String> = None;
    let mut content_codes: Vec<String> = Vec::new();

    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        match child {
            crate::parser::ast::Node::Element(e) if e.slot_name.as_deref() == Some("trigger") => {
                if is_iter {
                    return Err(CodegenError {
                        message: "AlertDialog trigger slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                if trigger_code.is_some() {
                    return Err(CodegenError {
                        message: "AlertDialog requires exactly one trigger slot (multiple found)"
                            .into(),
                        span: Some(elem.span),
                    });
                }
                trigger_code = Some(child_code);
            }
            crate::parser::ast::Node::Element(e) if e.slot_name.as_deref() == Some("footer") => {
                if is_iter {
                    return Err(CodegenError {
                        message: "AlertDialog footer slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                if footer_code.is_some() {
                    return Err(CodegenError {
                        message: "AlertDialog requires exactly one footer slot (multiple found)"
                            .into(),
                        span: Some(elem.span),
                    });
                }
                footer_code = Some(child_code);
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

    if let Some(tc) = trigger_code {
        code.push_str(&format!("\n            .trigger({})", tc));
    }
    if let Some(fc) = footer_code {
        code.push_str(&format!("\n            .footer({})", fc));
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

    fn make_text_child(text: &str) -> Node {
        Node::Text(text.into())
    }

    fn make_trigger() -> Element {
        Element {
            tag: "Button".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: "Delete".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        }
    }

    #[test]
    fn gen_alert_dialog_minimal() {
        let elem = make_element("AlertDialog", vec![], vec![]);
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::AlertDialog::new(cx)"));
    }

    #[test]
    fn gen_alert_dialog_with_title_and_description() {
        let elem = make_element(
            "AlertDialog",
            vec![
                Attribute::Static {
                    name: "title".into(),
                    value: "确认删除".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "description".into(),
                    value: "此操作不可撤销".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".title(\"确认删除\")"));
        assert!(code.contains(".description(\"此操作不可撤销\")"));
    }

    #[test]
    fn gen_alert_dialog_with_confirm() {
        let elem = make_element(
            "AlertDialog",
            vec![Attribute::Static {
                name: "confirm".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".confirm()"));
    }

    #[test]
    fn gen_alert_dialog_with_width() {
        let elem = make_element(
            "AlertDialog",
            vec![Attribute::Static {
                name: "width".into(),
                value: "420px".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".width(gpui::px(420.0))"));
    }

    #[test]
    fn gen_alert_dialog_with_trigger() {
        let elem = make_element("AlertDialog", vec![], vec![Node::Element(make_trigger())]);
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".trigger("));
    }

    #[test]
    fn gen_alert_dialog_with_content_children() {
        let elem = make_element(
            "AlertDialog",
            vec![],
            vec![make_text_child("First"), make_text_child("Second")],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert_eq!(code.matches(".child(").count(), 2);
        assert!(code.contains("\"First\""));
        assert!(code.contains("\"Second\""));
    }

    #[test]
    fn gen_alert_dialog_multiple_triggers_error() {
        let elem = make_element(
            "AlertDialog",
            vec![],
            vec![Node::Element(make_trigger()), Node::Element(make_trigger())],
        );
        let result = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("exactly one trigger"));
    }

    #[test]
    fn gen_alert_dialog_with_close_button_true() {
        let elem = make_element(
            "AlertDialog",
            vec![Attribute::Static {
                name: "close_button".into(),
                value: "true".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".close_button(true)"));
    }

    #[test]
    fn gen_alert_dialog_with_footer_slot() {
        let mut footer_btn = make_trigger();
        footer_btn.slot_name = Some("footer".into());
        footer_btn.attributes = vec![Attribute::Static {
            name: "label".into(),
            value: "自定义确认".into(),
            span: Span::empty(),
        }];
        let elem = make_element("AlertDialog", vec![], vec![Node::Element(footer_btn)]);
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".footer("));
    }

    #[test]
    fn gen_alert_dialog_multiple_footers_error() {
        let mut footer1 = make_trigger();
        footer1.slot_name = Some("footer".into());
        let mut footer2 = make_trigger();
        footer2.slot_name = Some("footer".into());
        let elem = make_element(
            "AlertDialog",
            vec![],
            vec![Node::Element(footer1), Node::Element(footer2)],
        );
        let result = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("exactly one footer"));
    }
}

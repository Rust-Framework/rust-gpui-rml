//! Dialog 构造代码生成
//!
//! ## 构造器
//!
//! `Dialog::new(cx: &mut App)` —— 直接使用 render 上下文的 `cx` 变量，不分配 ElementId。
//!
//! ## 子节点处理
//!
//! - `slot="trigger"` 的子元素 → `.trigger(element)`（同 HoverCard/Popover）
//! - 其余子元素 → `.child(element)` / `.children(iterator)`（ParentElement，同 Sheet）

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters::{event_setter, static_setter};

/// 生成 Dialog 构造代码
pub fn gen_dialog(
    elem: &Element,
    _ref_name: Option<&str>,
    _id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：Dialog::new(cx) —— 使用 render 上下文变量
    let mut code = "rml_ui::Dialog::new(cx)".to_string();

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "Dialog", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Dialog")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "Dialog",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = event_setter(name, handler) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Dialog")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：slot="trigger" → .trigger()，其余 → .child() / .children()
    let mut trigger_code: Option<String> = None;
    let mut content_codes: Vec<String> = Vec::new();

    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        match child {
            crate::parser::ast::Node::Element(e) if e.slot_name.as_deref() == Some("trigger") => {
                if is_iter {
                    return Err(CodegenError {
                        message: "Dialog trigger slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                if trigger_code.is_some() {
                    return Err(CodegenError {
                        message: "Dialog requires exactly one trigger slot (multiple found)".into(),
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

    fn make_text_child(text: &str) -> Node {
        Node::Text(text.into())
    }

    fn make_trigger() -> Element {
        Element {
            tag: "Button".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: "Open Dialog".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        }
    }

    #[test]
    fn gen_dialog_minimal() {
        let elem = make_element("Dialog", vec![], vec![]);
        let code = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Dialog::new(cx)"));
    }

    #[test]
    fn gen_dialog_with_title() {
        let elem = make_element(
            "Dialog",
            vec![Attribute::Static {
                name: "title".into(),
                value: "确认操作".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".title(\"确认操作\")"));
    }

    #[test]
    fn gen_dialog_with_width() {
        let elem = make_element(
            "Dialog",
            vec![Attribute::Static {
                name: "width".into(),
                value: "500px".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".width(gpui::px(500.0))"));
    }

    #[test]
    fn gen_dialog_with_overlay_false() {
        let elem = make_element(
            "Dialog",
            vec![Attribute::Static {
                name: "overlay".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".overlay(false)"));
    }

    #[test]
    fn gen_dialog_with_close_button_false() {
        let elem = make_element(
            "Dialog",
            vec![Attribute::Static {
                name: "close_button".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".close_button(false)"));
    }

    #[test]
    fn gen_dialog_with_trigger() {
        let elem = make_element("Dialog", vec![], vec![Node::Element(make_trigger())]);
        let code = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".trigger("));
    }

    #[test]
    fn gen_dialog_with_content_children() {
        let elem = make_element(
            "Dialog",
            vec![],
            vec![make_text_child("First"), make_text_child("Second")],
        );
        let code = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert_eq!(code.matches(".child(").count(), 2);
        assert!(code.contains("\"First\""));
        assert!(code.contains("\"Second\""));
    }

    #[test]
    fn gen_dialog_multiple_triggers_error() {
        let elem = make_element(
            "Dialog",
            vec![],
            vec![Node::Element(make_trigger()), Node::Element(make_trigger())],
        );
        let result = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("exactly one trigger"));
    }

    #[test]
    fn gen_dialog_with_footer() {
        let elem = make_element(
            "Dialog",
            vec![Attribute::Static {
                name: "footer".into(),
                value: "操作区域".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".footer(\"操作区域\")"));
    }
}

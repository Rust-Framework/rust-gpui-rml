//! Popover 构造代码生成
//!
//! ## 子节点处理
//!
//! - `slot="trigger"` 的子元素 → `.trigger(element)`（trigger 需实现 Selectable + IntoElement）
//! - 其余子元素 → `.child(element)`（content）

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters::{bind_setter, static_setter};

/// 生成 Popover 构造代码
pub fn gen_popover(
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
        format!("rml_ui::Popover::new({:?})", format!("rml_ref:{}", name))
    } else {
        format!("rml_ui::Popover::new((\"rml_el\", {}usize))", id_val)
    };

    // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
    append_css_class_styles(&mut code, elem, "Popover", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Popover")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = bind_setter(name, expr, &lv, &computed) {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "Popover",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Popover")
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
                        message: "Popover trigger slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                if trigger_code.is_some() {
                    return Err(CodegenError {
                        message: "Popover requires exactly one trigger slot (multiple found)".into(),
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

    #[test]
    fn gen_popover_minimal() {
        // <Popover><Button slot="trigger" label="Open" /></Popover>
        let trigger = Element {
            tag: "Button".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: "Open".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        };
        let elem = make_element(
            "Popover",
            vec![],
            vec![Node::Element(trigger)],
        );
        let code = gen_popover(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Popover::new((\"rml_el\", 0usize))"));
        assert!(code.contains(".trigger("));
        // trigger 内部是 Button
        assert!(code.contains("Button::new"));
    }

    #[test]
    fn gen_popover_with_content() {
        // <Popover><Button slot="trigger" label="Open" /><div>Content</div></Popover>
        let trigger = Element {
            tag: "Button".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: "Open".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        };
        let content = Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("Hello".into())],
            slot_name: None,
            ..Default::default()
        };
        let elem = make_element(
            "Popover",
            vec![],
            vec![Node::Element(trigger), Node::Element(content)],
        );
        let code = gen_popover(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".trigger("));
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_popover_with_anchor() {
        // <Popover anchor="bottom-right"><Button slot="trigger" label="Open" /></Popover>
        let trigger = Element {
            tag: "Button".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        };
        let elem = make_element(
            "Popover",
            vec![Attribute::Static {
                name: "anchor".into(),
                value: "bottom-right".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(trigger)],
        );
        let code = gen_popover(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".anchor(gpui::Anchor::BottomRight)"));
    }

    #[test]
    fn gen_popover_no_trigger() {
        // <Popover><div>Content</div></Popover> — 无 trigger slot
        let content = Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        };
        let elem = make_element("Popover", vec![], vec![Node::Element(content)]);
        let code = gen_popover(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // 无 trigger，但仍生成 content
        assert!(!code.contains(".trigger("));
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_popover_multiple_triggers_error() {
        // <Popover><Button slot="trigger" /><Button slot="trigger" /></Popover> — 多个 trigger
        let make_trigger = || Element {
            tag: "Button".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        };
        let elem = make_element(
            "Popover",
            vec![],
            vec![Node::Element(make_trigger()), Node::Element(make_trigger())],
        );
        let result = gen_popover(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("exactly one trigger"));
    }

    /// default_open 绑定属性生成 .default_open(self.field) 形式
    #[test]
    fn gen_popover_with_default_open_bind() {
        let trigger = Element {
            tag: "Button".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        };
        let elem = make_element(
            "Popover",
            vec![Attribute::Bind {
                name: "default_open".into(),
                expr: "is_open".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(trigger)],
        );
        let code = gen_popover(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".default_open(self.is_open)"));
    }
}

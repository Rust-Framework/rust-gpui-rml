//! Stepper 容器 codegen —— 构造 + 属性 + 子节点 `.item(StepperItem::new()...)` 注入。
//!
//! 将 `<Stepper><step-item icon="Check">步骤一</step-item></Stepper>` 转译为
//! `rml_ui::Stepper::new(id).selected_index(0usize).item(rml_ui::StepperItem::new().icon(...).child("步骤一"))`。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::twoway;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 生成 Stepper 构造代码（构造 + 属性 + 子节点 .item(StepperItem) 注入）
///
/// 由 `StepperTranslator` 调用。
pub fn gen_stepper(
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
        format!("rml_ui::Stepper::new({:?})", format!("rml_ref:{}", name))
    } else {
        format!("rml_ui::Stepper::new((\"rml_el\", {}usize))", id_val)
    };

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "Stepper", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter（先调 stepper 专用 setter，未命中回退到公共 setter）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 双向绑定检测：selected_index={field} → 自动双向（on_click &usize 回写）
    let twoway_on_click = twoway::detect_twoway_binding(elem, "Stepper")
        .and_then(|(field, spec, user_handler)| twoway::gen_twoway_on_click(spec, &field, user_handler));

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value, "Stepper") {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Stepper")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "Stepper")
                {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "Stepper",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                // 双向绑定接管 on_click 时跳过正常 event setter
                if name == "on_click" && twoway_on_click.is_some() {
                    continue;
                }
                if let Some(s) = super::setters::event_setter(name, handler, "Stepper") {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Stepper")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 注入双向绑定的合并 on_click 回调
    if let Some(on_click_code) = twoway_on_click {
        code.push_str(&on_click_code);
    }

    // 3. 子节点 → .item(rml_ui::StepperItem::new()...)
    for child in &elem.children {
        match child {
            Node::Element(child_elem) if tags::is_item_builder_tag(&child_elem.tag) => {
                let item_code = gen_stepper_item(child_elem, ctx, id_counter, loop_vars)?;
                code.push_str(&format!("\n            .item({})", item_code));
            }
            Node::Text(text) => {
                eprintln!(
                    "[rml warning] <Stepper> 不支持文本子节点 {:?}，已忽略",
                    text
                );
            }
            Node::Element(child_elem) => {
                return Err(CodegenError {
                    message: format!(
                        "<Stepper> 仅支持 <StepperItem> 或 <step-item> 子节点，得到 <{}>",
                        child_elem.tag
                    ),
                    span: Some(elem.span),
                });
            }
            _ => {}
        }
    }

    Ok(code)
}

/// 生成 StepperItem 构造代码
///
/// 生成形如：`rml_ui::StepperItem::new().icon(rml_ui::Icon::new(rml_ui::IconName::Check)).child("步骤一")`
fn gen_stepper_item(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut code = String::from("rml_ui::StepperItem::new()");

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value, "StepperItem") {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_static_setter(
                    name, value, "StepperItem",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "StepperItem")
                {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "StepperItem",
                ) {
                    code.push_str(&s);
                }
            }
            _ => {}
        }
    }

    // 子节点 → .child(...) / .children(...)（StepperItem 实现 ParentElement）
    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!(".children({})", child_code));
        } else {
            code.push_str(&format!(".child({})", child_code));
        }
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Directive, Element, EventHandler, Node};
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

    fn make_element_with_directives(
        tag: &str,
        attrs: Vec<Attribute>,
        directives: Vec<Directive>,
        children: Vec<Node>,
    ) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives,
            children,
            slot_name: None,
            ..Default::default()
        }
    }

    #[test]
    fn gen_stepper_minimal() {
        let elem = make_element("Stepper", vec![], vec![]);
        let mut id = 0;
        let code = gen_stepper(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Stepper::new"));
        assert!(code.contains("\"rml_el\""));
    }

    #[test]
    fn gen_stepper_with_vertical() {
        let elem = make_element(
            "Stepper",
            vec![Attribute::Static {
                name: "direction".into(),
                value: "vertical".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_stepper(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".vertical()"));
    }

    #[test]
    fn gen_stepper_with_selected_index() {
        let elem = make_element(
            "Stepper",
            vec![Attribute::Static {
                name: "selected_index".into(),
                value: "2".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_stepper(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".selected_index(2usize)"));
    }

    #[test]
    fn gen_stepper_with_step_item() {
        let item = make_element(
            "StepperItem",
            vec![Attribute::Static {
                name: "icon".into(),
                value: "Check".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("步骤一".into())],
        );
        let stepper = make_element("Stepper", vec![], vec![Node::Element(item)]);
        let mut id = 0;
        let code = gen_stepper(&stepper, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".item("));
        assert!(code.contains("rml_ui::StepperItem::new()"));
        assert!(code.contains(".icon(rml_ui::Icon::new(rml_ui::IconName::Check))"));
        assert!(code.contains(".child(\"步骤一\")"));
    }

    #[test]
    fn gen_stepper_with_step_item_short_form() {
        let item = make_element(
            "step-item",
            vec![],
            vec![Node::Text("步骤一".into())],
        );
        let stepper = make_element("stepper", vec![], vec![Node::Element(item)]);
        let mut id = 0;
        let code = gen_stepper(&stepper, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".item("));
        assert!(code.contains("rml_ui::StepperItem::new()"));
        assert!(code.contains(".child(\"步骤一\")"));
    }

    #[test]
    fn gen_stepper_with_on_click() {
        let elem = make_element(
            "Stepper",
            vec![Attribute::Event {
                name: "on_click".into(),
                handler: EventHandler::Ident("on_step_click".into()),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_stepper(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".on_click("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("idx: &usize"));
        assert!(code.contains("this.on_step_click(idx, cx)"));
    }

    #[test]
    fn gen_stepper_with_ref_uses_stable_id() {
        let elem = make_element_with_directives(
            "Stepper",
            vec![],
            vec![Directive::Ref { name: "my_stepper".into(), span: Span::empty() }],
            vec![],
        );
        let mut id = 0;
        let code = gen_stepper(
            &elem,
            Some("my_stepper"),
            id,
            &ctx(),
            &mut id,
            &Vec::new(),
            &[],
        )
        .unwrap();
        assert!(code.contains("rml_ui::Stepper::new(\"rml_ref:my_stepper\")"));
        assert!(!code.contains("rml_el"));
    }

    #[test]
    fn gen_stepper_rejects_non_item_child() {
        let div = make_element("div", vec![], vec![]);
        let stepper = make_element("Stepper", vec![], vec![Node::Element(div)]);
        let mut id = 0;
        let result = gen_stepper(&stepper, None, id, &ctx(), &mut id, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("仅支持 <StepperItem>"));
    }

    #[test]
    fn gen_stepper_with_sizable() {
        let elem = make_element(
            "Stepper",
            vec![Attribute::Static {
                name: "size".into(),
                value: "small".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_stepper(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".with_size(rml_ui::Size::Small)"));
    }

    #[test]
    fn gen_stepper_with_text_center() {
        let elem = make_element(
            "Stepper",
            vec![Attribute::Static {
                name: "text_center".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_stepper(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".text_center(true)"));
    }
}

//! Label 组件代码生成
//!
//! Label 构造器接受 label 文本作为参数：`Label::new(label: impl Into<SharedString>)`
//! 不使用 ElementId。本模块从 `label="..."` 属性或文本子节点提取文本，生成构造调用。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};

/// 生成 Label 构造代码
///
/// 从 `label="..."` 静态属性、`label={expr}` 绑定属性或文本子节点提取 label 文本，
/// 生成 `rml_ui::Label::new(text)` 调用。
pub fn gen_label(
    elem: &Element,
    ctx: &CodegenCtx,
    _id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let resolved = "Label";
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. 构造器：从 label 属性或文本子节点提取文本
    let mut code = String::new();
    let mut label_set = false;

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == "label" => {
                code.push_str(&format!("rml_ui::Label::new({:?})", value));
                label_set = true;
            }
            Attribute::Bind { name, expr, .. } if name == "label" => {
                let rust_expr =
                    super::component::component_bind_rust_expr(expr, &lv, &computed);
                code.push_str(&format!("rml_ui::Label::new({})", rust_expr));
                label_set = true;
            }
            _ => {}
        }
    }

    // 回退到文本子节点
    if !label_set {
        let mut text = String::new();
        for child in &elem.children {
            if let Node::Text(t) = child {
                text = t.clone();
                break;
            }
        }
        code.push_str(&format!("rml_ui::Label::new({:?})", text));
    }

    // 2. 其他属性 → builder 方法（跳过 label，已用于构造器）
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if name == "label" {
                    continue;
                }
                if let Some(setter) =
                    super::component::component_static_setter(name, value, resolved)
                {
                    code.push_str(&setter);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if name == "label" {
                    continue;
                }
                if let Some(setter) = super::component::component_bind_setter(
                    name, expr, &lv, &computed, resolved,
                ) {
                    code.push_str(&setter);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(setter) =
                    super::component::component_event_setter(name, handler, resolved)
                {
                    code.push_str(&setter);
                }
            }
        }
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::Element;
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            ..Default::default()
        }
    }

    fn make_element(attrs: Vec<Attribute>, children: Vec<Node>) -> Element {
        Element {
            tag: "Label".into(),
            attributes: attrs,
            directives: vec![],
            children,
            slot_name: None,
            ..Default::default()
        }
    }

    #[test]
    fn gen_label_from_static_attr() {
        let elem = make_element(
            vec![Attribute::Static {
                name: "label".into(),
                value: "Hello".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_label(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Label::new(\"Hello\")"));
    }

    #[test]
    fn gen_label_from_text_child() {
        let elem = make_element(vec![], vec![Node::Text("World".into())]);
        let mut id = 0;
        let code = gen_label(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Label::new(\"World\")"));
    }

    #[test]
    fn gen_label_from_bind() {
        let elem = make_element(
            vec![Attribute::Bind {
                name: "label".into(),
                expr: "title".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_label(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Label::new(self.title)"));
    }
}

//! Field 构造代码生成
//!
//! ## 构造器
//!
//! `Field::new()` —— 无 ElementId、无 cx 参数（RenderOnce + ParentElement）。
//!
//! ## 子节点处理
//!
//! Field 实现 `ParentElement`，子节点通过 `.child()` / `.children()` 注入。
//! 典型子节点：Input、Switch、Checkbox、Select 等表单控件。
//!
//! ## 属性
//!
//! - `label="用户名"` → `.label("用户名")`
//! - `description="帮助文本"` → `.description("帮助文本")`
//! - `required` → `.required(true)`（独立布尔属性，默认 false）
//! - `visible="false"` → `.visible(false)`（默认 true，显式 false 隐藏）
//! - `col_span="2"` → `.col_span(2)`
//! - `col_start="1"` → `.col_start(1)`
//! - `col_end="3"` → `.col_end(3)`

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters::static_setter;

/// 生成 Field 构造代码
pub fn gen_field(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：Field::new()（无 ElementId、无 cx）
    let mut code = "rml_ui::Field::new()".to_string();

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "Field", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Field")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> =
                    ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name,
                    expr,
                    &lv,
                    &computed,
                    "Field",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Field")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：.child() / .children()（ParentElement）
    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!("\n            .children({})", child_code));
        } else {
            code.push_str(&format!("\n            .child({})", child_code));
        }
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
            ..Default::default()
        }
    }

    #[test]
    fn gen_field_minimal() {
        let elem = make_element("Field", vec![], vec![]);
        let code = gen_field(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Field::new()"));
    }

    #[test]
    fn gen_field_with_label() {
        let elem = make_element(
            "Field",
            vec![Attribute::Static {
                name: "label".into(),
                value: "用户名".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_field(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".label(\"用户名\")"));
    }

    #[test]
    fn gen_field_with_description() {
        let elem = make_element(
            "Field",
            vec![Attribute::Static {
                name: "description".into(),
                value: "请输入用户名".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_field(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".description(\"请输入用户名\")"));
    }

    #[test]
    fn gen_field_required() {
        let elem = make_element(
            "Field",
            vec![Attribute::Static {
                name: "required".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_field(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".required(true)"));
    }

    #[test]
    fn gen_field_visible_false() {
        let elem = make_element(
            "Field",
            vec![Attribute::Static {
                name: "visible".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_field(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".visible(false)"));
    }

    #[test]
    fn gen_field_col_span() {
        let elem = make_element(
            "Field",
            vec![Attribute::Static {
                name: "col_span".into(),
                value: "2".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_field(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".col_span(2)"));
    }

    #[test]
    fn gen_field_with_children() {
        let elem = make_element(
            "Field",
            vec![],
            vec![Node::Text("input_element".into())],
        );
        let code = gen_field(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_field_full_example() {
        let elem = make_element(
            "Field",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "密码".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "required".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "description".into(),
                    value: "至少 8 位".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_field(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Field::new()"));
        assert!(code.contains(".label(\"密码\")"));
        assert!(code.contains(".required(true)"));
        assert!(code.contains(".description(\"至少 8 位\")"));
    }
}

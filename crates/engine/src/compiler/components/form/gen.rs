//! Form 构造代码生成
//!
//! ## 构造器
//!
//! - 默认：`Form::vertical()`（无 ElementId、无 cx 参数）
//! - `horizontal` 属性：`Form::horizontal()`
//! - `vertical` 属性：`Form::vertical()`（显式，与默认相同）
//!
//! ## 子节点处理
//!
//! Form 的 `.child()` 方法接受 `impl Into<Field>`，非 Field 子节点会编译失败。
//! 代码生成器为每个非空白子节点生成 `.child(child_code)`。
//! 空白文本节点被忽略（避免 `&str` 不满足 `Into<Field>` 的编译错误）。
//!
//! ## 属性
//!
//! - `label_width="200"` → `.label_width(gpui::px(200.))`
//! - `label_text_size="0.875"` → `.label_text_size(gpui::rems(0.875))`
//! - `columns="2"` → `.columns(2)`
//! - `horizontal` / `vertical` → 构造器选择

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};

use super::setters::{form_variant_from_attr, static_setter};

/// 生成 Form 构造代码
pub fn gen_form(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：默认 vertical，horizontal 属性切换
    let mut is_horizontal = false;
    let mut is_vertical_explicit = false;

    for attr in &elem.attributes {
        if let Attribute::Static { name, value, .. } = attr {
            if form_variant_from_attr(name).is_some()
                && (value.is_empty() || value.eq_ignore_ascii_case("true"))
            {
                if name == "horizontal" {
                    is_horizontal = true;
                } else if name == "vertical" {
                    is_vertical_explicit = true;
                }
            }
        }
    }

    let mut code = if is_horizontal {
        "rml_ui::Form::horizontal()".to_string()
    } else {
        // vertical 是默认，vertical 属性显式时也用 Form::vertical()
        let _ = is_vertical_explicit; // 显式 vertical 与默认相同
        "rml_ui::Form::vertical()".to_string()
    };

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "Form", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter（跳过 variant 属性，已处理）
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                // variant 属性已在构造器处理，跳过
                if form_variant_from_attr(name).is_some() {
                    continue;
                }
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Form")
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
                    "Form",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Form")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：.child(field_code)，忽略空白文本
    for child in &elem.children {
        // 忽略空白文本节点（Form 的 .child() 只接受 impl Into<Field>）
        if let Node::Text(t) = child {
            if t.trim().is_empty() {
                continue;
            }
        }
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

    fn make_field(label: &str) -> Element {
        Element {
            tag: "Field".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: label.into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn gen_form_default_vertical() {
        let elem = make_element("Form", vec![], vec![]);
        let code = gen_form(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Form::vertical()"));
    }

    #[test]
    fn gen_form_horizontal() {
        let elem = make_element(
            "Form",
            vec![Attribute::Static {
                name: "horizontal".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_form(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Form::horizontal()"));
    }

    #[test]
    fn gen_form_vertical_explicit() {
        let elem = make_element(
            "Form",
            vec![Attribute::Static {
                name: "vertical".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_form(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Form::vertical()"));
    }

    #[test]
    fn gen_form_with_label_width() {
        let elem = make_element(
            "Form",
            vec![Attribute::Static {
                name: "label_width".into(),
                value: "200".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_form(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".label_width(gpui::px(200.))"));
    }

    #[test]
    fn gen_form_with_columns() {
        let elem = make_element(
            "Form",
            vec![Attribute::Static {
                name: "columns".into(),
                value: "2".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_form(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".columns(2)"));
    }

    #[test]
    fn gen_form_with_field_children() {
        let elem = make_element(
            "Form",
            vec![],
            vec![
                Node::Element(make_field("Name")),
                Node::Element(make_field("Email")),
            ],
        );
        let code = gen_form(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert_eq!(code.matches(".child(").count(), 2);
    }

    #[test]
    fn gen_form_ignores_whitespace_text() {
        let elem = make_element(
            "Form",
            vec![],
            vec![
                Node::Text("  \n  ".into()),
                Node::Element(make_field("Name")),
                Node::Text("  ".into()),
            ],
        );
        let code = gen_form(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // 只有 1 个 .child（Field），空白文本被忽略
        assert_eq!(code.matches(".child(").count(), 1);
    }

    #[test]
    fn gen_form_full_example() {
        let elem = make_element(
            "Form",
            vec![
                Attribute::Static {
                    name: "horizontal".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "label_width".into(),
                    value: "120".into(),
                    span: Span::empty(),
                },
            ],
            vec![Node::Element(make_field("Username"))],
        );
        let code = gen_form(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Form::horizontal()"));
        assert!(code.contains(".label_width(gpui::px(120.))"));
        assert!(code.contains(".child("));
    }
}

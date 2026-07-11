//! Markdown 构造代码生成
//!
//! ## 构造器
//!
//! `Markdown::new()` —— 无 ElementId、无 cx（RenderOnce 组件）。
//!
//! ## 属性
//!
//! - `content` (static/bind) → `.content("text")` / `.content(self.field)`
//!
//! `content` 由本函数内联处理，不走 setter 链路。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};
use crate::compiler::setters::{
    component_bind_rust_expr, component_bind_setter, component_event_setter,
    component_static_setter,
};

/// 生成 Markdown 构造代码
pub fn gen_markdown(
    elem: &Element,
    ctx: &CodegenCtx,
    _id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let mut code = "rml_ui::Markdown::new()".to_string();

    append_css_class_styles(&mut code, elem, "Markdown", ctx.stylesheet.as_ref(), parents);

    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        // content 由本函数内联处理，跳过 setter 链路
        let is_content = match attr {
            Attribute::Static { name, .. } | Attribute::Bind { name, .. } => name == "content",
            _ => false,
        };
        if is_content {
            match attr {
                Attribute::Static { value, .. } => {
                    code.push_str(&format!(".content({:?})", value));
                }
                Attribute::Bind { expr, .. } => {
                    let rust_expr = component_bind_rust_expr(expr, &lv, &computed);
                    code.push_str(&format!(".content({})", rust_expr));
                }
                _ => {}
            }
            continue;
        }

        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = component_static_setter(name, value, "Markdown") {
                    code.push_str(&s);
                } else {
                    crate::compiler::setters::check_missing_mapping(
                        ctx, "Markdown", name, "static",
                    )?;
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    component_bind_setter(name, expr, &lv, &computed, "Markdown")
                {
                    code.push_str(&s);
                } else {
                    crate::compiler::setters::check_missing_mapping(
                        ctx, "Markdown", name, "bind",
                    )?;
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = component_event_setter(name, handler, "Markdown") {
                    code.push_str(&s);
                }
            }
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

    fn make_element(tag: &str, attrs: Vec<Attribute>) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives: vec![],
            children: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn gen_markdown_minimal() {
        let elem = make_element("Markdown", vec![]);
        let code = gen_markdown(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Markdown::new()"));
    }

    #[test]
    fn gen_markdown_static_content() {
        let elem = make_element(
            "Markdown",
            vec![Attribute::Static {
                name: "content".into(),
                value: "# Hello\n\n**bold** text".into(),
                span: Span::empty(),
            }],
        );
        let code = gen_markdown(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Markdown::new()"));
        assert!(code.contains(r##".content("# Hello\n\n**bold** text")"##));
    }

    #[test]
    fn gen_markdown_bind_content() {
        let mut c = ctx();
        c.computed_methods = vec!["markdown_text".into()];
        let elem = make_element(
            "Markdown",
            vec![Attribute::Bind {
                name: "content".into(),
                expr: "markdown_text".into(),
                span: Span::empty(),
            }],
        );
        let code = gen_markdown(&elem, &c, &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Markdown::new()"));
        assert!(code.contains(".content(self.markdown_text())"));
    }

    #[test]
    fn gen_markdown_bind_content_field() {
        let elem = make_element(
            "Markdown",
            vec![Attribute::Bind {
                name: "content".into(),
                expr: "raw_text".into(),
                span: Span::empty(),
            }],
        );
        let code = gen_markdown(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".content(self.raw_text)"));
    }

    #[test]
    fn gen_markdown_with_style() {
        let elem = make_element(
            "Markdown",
            vec![
                Attribute::Static {
                    name: "content".into(),
                    value: "Hello".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "padding".into(),
                    value: "16px".into(),
                    span: Span::empty(),
                },
            ],
        );
        let code = gen_markdown(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".content(\"Hello\")"));
        assert!(code.contains(".p(gpui::px(16.0))"));
    }

    #[test]
    fn gen_markdown_ai_chat_example() {
        let mut c = ctx();
        c.computed_methods = vec!["ai_response".into()];
        let elem = make_element(
            "Markdown",
            vec![
                Attribute::Bind {
                    name: "content".into(),
                    expr: "ai_response".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "padding".into(),
                    value: "12px".into(),
                    span: Span::empty(),
                },
            ],
        );
        let code = gen_markdown(&elem, &c, &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Markdown::new()"));
        assert!(code.contains(".content(self.ai_response())"));
        assert!(code.contains(".p(gpui::px(12.0))"));
    }

    #[test]
    fn gen_markdown_no_children() {
        let elem = Element {
            tag: "Markdown".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("should be ignored".into())],
            ..Default::default()
        };
        let code = gen_markdown(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // Markdown 不处理子节点（content 通过属性传入）
        assert!(!code.contains(".child("));
        assert!(!code.contains("should be ignored"));
    }
}

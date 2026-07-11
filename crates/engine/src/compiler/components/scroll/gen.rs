//! Scroll 构造代码生成
//!
//! ## 构造器
//!
//! `Scroll::new()` —— 无 ElementId、无 cx 参数（RenderOnce 组件）。
//!
//! ## 子节点处理
//!
//! 所有子节点通过 `.child()` / `.children()` 注入（ParentElement）。
//!
//! ## variant 布尔属性
//!
//! `vertical` / `horizontal` / `both` → `.vertical()` / `.horizontal()` / `.both()`（独立布尔属性）

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters::static_setter;

/// 生成 Scroll 构造代码
pub fn gen_scroll(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：Scroll::new()（无 ElementId、无 cx）
    let mut code = "rml_ui::Scroll::new()".to_string();

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "Scroll", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Scroll")
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
                    "Scroll",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Scroll")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：全部通过 .child() / .children() 注入（ParentElement）
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

    fn make_text_child(text: &str) -> Node {
        Node::Text(text.into())
    }

    #[test]
    fn gen_scroll_minimal() {
        let elem = make_element("Scroll", vec![], vec![]);
        let code = gen_scroll(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Scroll::new()"));
    }

    #[test]
    fn gen_scroll_vertical() {
        let elem = make_element(
            "Scroll",
            vec![Attribute::Static {
                name: "vertical".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_scroll(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".vertical()"));
    }

    #[test]
    fn gen_scroll_horizontal() {
        let elem = make_element(
            "Scroll",
            vec![Attribute::Static {
                name: "horizontal".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_scroll(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".horizontal()"));
    }

    #[test]
    fn gen_scroll_both() {
        let elem = make_element(
            "Scroll",
            vec![Attribute::Static {
                name: "both".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_scroll(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".both()"));
    }

    #[test]
    fn gen_scroll_with_children() {
        let elem = make_element(
            "Scroll",
            vec![],
            vec![make_text_child("First"), make_text_child("Second")],
        );
        let code = gen_scroll(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert_eq!(code.matches(".child(").count(), 2);
        assert!(code.contains("\"First\""));
        assert!(code.contains("\"Second\""));
    }

    #[test]
    fn gen_scroll_default_is_vertical() {
        let elem = make_element("Scroll", vec![], vec![]);
        let code = gen_scroll(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Scroll::new()"));
        // 不设置任何 variant 时，Scroll::new() 默认 vertical，无额外方法调用
    }

    #[test]
    fn gen_scroll_full_example() {
        let elem = make_element(
            "Scroll",
            vec![Attribute::Static {
                name: "both".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![make_text_child("Content")],
        );
        let code = gen_scroll(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Scroll::new()"));
        assert!(code.contains(".both()"));
        assert!(code.contains(".child("));
        assert!(code.contains("\"Content\""));
    }
}

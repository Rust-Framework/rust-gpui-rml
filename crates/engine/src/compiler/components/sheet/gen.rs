//! Sheet 构造代码生成
//!
//! ## 构造器
//!
//! `Sheet::new(_: &mut Window, cx: &mut App)` —— 直接使用 render 上下文的
//! `_window` 和 `cx` 变量，不分配 ElementId。
//!
//! ## 子节点处理
//!
//! Sheet 实现 `ParentElement`，所有子节点通过 `.child()` / `.children()` 注入为 content。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters::{event_setter, static_setter};

/// 生成 Sheet 构造代码
pub fn gen_sheet(
    elem: &Element,
    _ref_name: Option<&str>,
    _id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：Sheet::new(_window, cx) —— 使用 render 上下文变量
    let mut code = "rml_ui::Sheet::new(_window, cx)".to_string();

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "Sheet", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Sheet")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "Sheet",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = event_setter(name, handler) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Sheet")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：全部通过 .child() / .children() 注入
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
            slot_name: None,
            ..Default::default()
        }
    }

    fn make_text_child(text: &str) -> Node {
        Node::Text(text.into())
    }

    fn make_div_child() -> Node {
        Node::Element(Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("Content".into())],
            slot_name: None,
            ..Default::default()
        })
    }

    #[test]
    fn gen_sheet_minimal() {
        let elem = make_element("Sheet", vec![], vec![]);
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Sheet::new(_window, cx)"));
    }

    #[test]
    fn gen_sheet_with_title() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "title".into(),
                value: "设置面板".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".title(\"设置面板\")"));
    }

    #[test]
    fn gen_sheet_with_size() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "size".into(),
                value: "400px".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".size(gpui::px(400.0))"));
    }

    #[test]
    fn gen_sheet_with_resizable_false() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "resizable".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".resizable(false)"));
    }

    #[test]
    fn gen_sheet_with_children() {
        let elem = make_element("Sheet", vec![], vec![make_div_child()]);
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_sheet_with_multiple_children() {
        let elem = make_element(
            "Sheet",
            vec![],
            vec![make_text_child("First"), make_text_child("Second")],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // Two .child() calls for text children (text nodes don't have nested .child())
        assert_eq!(code.matches(".child(").count(), 2);
        assert!(code.contains("\"First\""));
        assert!(code.contains("\"Second\""));
    }

    #[test]
    fn gen_sheet_with_overlay_false() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "overlay".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".overlay(false)"));
    }

    #[test]
    fn gen_sheet_with_footer() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "footer".into(),
                value: "操作栏".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".footer(\"操作栏\")"));
    }
}

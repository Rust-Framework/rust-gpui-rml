//! Grid / GridItem 构造代码生成
//!
//! ## 构造器
//!
//! - `Grid::new()` / `GridItem::new()` —— 无 ElementId、无 cx（RenderOnce 组件）
//!
//! ## 子节点处理
//!
//! 两者均实现 `ParentElement`，子节点通过 `.child()` / `.children()` 注入。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters::{grid_item_static_setter, grid_static_setter};

/// 生成 Grid 构造代码
pub fn gen_grid(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let mut code = "rml_ui::Grid::new()".to_string();

    append_css_class_styles(&mut code, elem, "Grid", ctx.stylesheet.as_ref(), parents);

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = grid_static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Grid")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> =
                    ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "Grid",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Grid")
                {
                    code.push_str(&s);
                }
            }
        }
    }

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

/// 生成 GridItem 构造代码
pub fn gen_grid_item(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let mut code = "rml_ui::GridItem::new()".to_string();

    append_css_class_styles(
        &mut code,
        elem,
        "GridItem",
        ctx.stylesheet.as_ref(),
        parents,
    );

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = grid_item_static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "GridItem")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> =
                    ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "GridItem",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "GridItem")
                {
                    code.push_str(&s);
                }
            }
        }
    }

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
    fn gen_grid_minimal() {
        let elem = make_element("Grid", vec![], vec![]);
        let code = gen_grid(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Grid::new()"));
    }

    #[test]
    fn gen_grid_with_columns() {
        let elem = make_element(
            "Grid",
            vec![Attribute::Static {
                name: "columns".into(),
                value: "3".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_grid(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".columns(3u16)"));
    }

    #[test]
    fn gen_grid_with_rows() {
        let elem = make_element(
            "Grid",
            vec![Attribute::Static {
                name: "rows".into(),
                value: "2".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_grid(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".rows(2u16)"));
    }

    #[test]
    fn gen_grid_with_children() {
        let elem = make_element(
            "Grid",
            vec![],
            vec![Node::Text("Cell".into())],
        );
        let code = gen_grid(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_grid_item_minimal() {
        let elem = make_element("GridItem", vec![], vec![]);
        let code = gen_grid_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::GridItem::new()"));
    }

    #[test]
    fn gen_grid_item_col_span() {
        let elem = make_element(
            "GridItem",
            vec![Attribute::Static {
                name: "col_span".into(),
                value: "2".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_grid_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".col_span(2u16)"));
    }

    #[test]
    fn gen_grid_item_full() {
        let elem = make_element(
            "GridItem",
            vec![
                Attribute::Static {
                    name: "col_span".into(),
                    value: "2".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "row_start".into(),
                    value: "1".into(),
                    span: Span::empty(),
                },
            ],
            vec![Node::Text("Content".into())],
        );
        let code = gen_grid_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".col_span(2u16)"));
        assert!(code.contains(".row_start(1i16)"));
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_grid_ide_layout() {
        // 模拟 IDE 布局：Grid columns="3" + GridItem col-span="2"
        let elem = make_element(
            "Grid",
            vec![Attribute::Static {
                name: "columns".into(),
                value: "3".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(make_element(
                "GridItem",
                vec![Attribute::Static {
                    name: "col_span".into(),
                    value: "2".into(),
                    span: Span::empty(),
                }],
                vec![Node::Text("Editor".into())],
            ))],
        );
        let code = gen_grid(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Grid::new()"));
        assert!(code.contains(".columns(3u16)"));
        assert!(code.contains("rml_ui::GridItem::new()"));
        assert!(code.contains(".col_span(2u16)"));
    }
}

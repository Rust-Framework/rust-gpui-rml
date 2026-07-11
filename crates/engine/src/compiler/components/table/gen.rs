//! Table 容器 codegen —— 构造 + 属性 + Column 子节点 + template slot 子节点。
//!
//! 将 `<Table ...><Column ... /><template slot="...">...</template></Table>`
//! 转译为 `rml_ui::Table::new(id).<setters>.column(...).header_template(...)...`。
//!
//! 参考 `accordion/gen.rs`，与 Accordion 的区别：
//! - Column 子节点生成直接构造表达式（非闭包），调用 `.column(...)`
//! - 额外处理 `<template slot="header/cell/footer">` 插槽模板子节点

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 生成 Table 构造代码（构造 + 属性 + Column 子节点 + template slot 子节点）
///
/// 由 `TableTranslator` 调用。
pub fn gen_table(
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
        format!("rml_ui::Table::new({:?})", format!("rml_ref:{}", name))
    } else {
        format!("rml_ui::Table::new((\"rml_el\", {}usize))", id_val)
    };

    // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
    append_css_class_styles(&mut code, elem, "Table", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter（先调 table 专用 setter，未命中回退到公共 setter）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value, "Table") {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Table")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "Table")
                {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "Table",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Table")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点处理：Column 子标签 + template slot 子标签
    for child in &elem.children {
        match child {
            // <Column> / <column> 子标签 → .column(TableColumn::new(...))
            Node::Element(child_elem) if tags::is_item_builder_tag(&child_elem.tag) => {
                let column_code =
                    super::column::gen_column(child_elem, ctx, id_counter, loop_vars)?;
                code.push_str(&format!("\n            .column({})", column_code));
            }
            // <template slot="header/cell/footer"> → .header_template(...) / .cell_template(...) / .footer_template(...)
            Node::Element(child_elem)
                if child_elem.tag == "template" && child_elem.slot_name.is_some() =>
            {
                let template_code =
                    super::template::gen_template(child_elem, ctx, id_counter, loop_vars)?;
                code.push_str(&format!("\n            {}", template_code));
            }
            // 文本子节点 → 警告并忽略
            Node::Text(text) => {
                eprintln!(
                    "[rml warning] <Table> 不支持文本子节点 {:?}，已忽略",
                    text
                );
            }
            // 其他元素子节点 → 报错
            Node::Element(child_elem) => {
                return Err(CodegenError {
                    message: format!(
                        "<table> 仅支持 <Column> 或 <template slot=\"...\"> 子节点，得到 <{}>",
                        child_elem.tag
                    ),
                    span: Some(elem.span),
                });
            }
            _ => {}
        }
    }

    // 4. 自动注入 .notify(...) 回调 —— 使用 cx.weak_entity() 桥接，
    //    使 delegate 在编辑状态变更时能触发 ViewModel 重新渲染。
    code.push_str(
        "\n            .notify(std::rc::Rc::new({\n                \
         let weak = cx.weak_entity();\n                \
         move |app: &mut gpui::App| {\n                    \
         if let Some(entity) = weak.upgrade() {\n                        \
         entity.update(app, |_, cx| cx.notify());\n                    \
         }\n                \
         }\n            }))",
    );

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Directive, Element, Node};
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
    fn gen_table_minimal() {
        // <Table /> → rml_ui::Table::new(("rml_el", 0usize))
        let elem = make_element("Table", vec![], vec![]);
        let mut id = 0;
        let code = gen_table(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Table::new"));
        assert!(code.contains("\"rml_el\""));
    }

    #[test]
    fn gen_table_lowercase_tag() {
        // <table /> 小写标签（由 canonical_tag 处理，gen_table 本身不检查 tag）
        let elem = make_element("table", vec![], vec![]);
        let mut id = 0;
        let code = gen_table(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Table::new"));
    }

    #[test]
    fn gen_table_with_bordered_and_stripe() {
        // <Table bordered="" stripe="" /> → .bordered(true).stripe(true)
        let elem = make_element(
            "Table",
            vec![
                Attribute::Static { name: "bordered".into(), value: "".into(), span: Span::empty() },
                Attribute::Static { name: "stripe".into(), value: "".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_table(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".bordered(true)"));
        assert!(code.contains(".stripe(true)"));
    }

    #[test]
    fn gen_table_with_bind_columns_and_rows() {
        // <Table columns={api_columns} rows={api_rows} /> → .columns(...).rows(...)
        let elem = make_element(
            "Table",
            vec![
                Attribute::Bind { name: "columns".into(), expr: "api_columns".into(), span: Span::empty() },
                Attribute::Bind { name: "rows".into(), expr: "api_rows".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_table(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".columns(self.api_columns.clone())"));
        assert!(code.contains(".rows(self.api_rows.clone())"));
    }

    #[test]
    fn gen_table_with_column_child() {
        // <Table><Column key="name" title="Name" /></Table> → .column(TableColumn::new("name", "Name"))
        let column = make_element(
            "Column",
            vec![
                Attribute::Static { name: "key".into(), value: "name".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "Name".into(), span: Span::empty() },
            ],
            vec![],
        );
        let table = make_element("Table", vec![], vec![Node::Element(column)]);
        let mut id = 0;
        let code = gen_table(&table, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".column("));
        assert!(code.contains("rml_ui::TableColumn::new(\"name\", \"Name\")"));
    }

    #[test]
    fn gen_table_with_column_child_lowercase() {
        // <table><column key="x" title="X" /></table>
        let column = make_element(
            "column",
            vec![
                Attribute::Static { name: "key".into(), value: "x".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "X".into(), span: Span::empty() },
            ],
            vec![],
        );
        let table = make_element("table", vec![], vec![Node::Element(column)]);
        let mut id = 0;
        let code = gen_table(&table, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".column("));
        assert!(code.contains("rml_ui::TableColumn::new(\"x\", \"X\")"));
    }

    #[test]
    fn gen_table_with_header_template() {
        // <Table><template slot="header"><span>H</span></template></Table>
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("H".into())],
            slot_name: None,
            ..Default::default()
        };
        let template = Element {
            tag: "template".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Element(span)],
            slot_name: Some("header".to_string()),
            ..Default::default()
        };
        let table = make_element("Table", vec![], vec![Node::Element(template)]);
        let mut id = 0;
        let code = gen_table(&table, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".header_template("));
        assert!(code.contains("std::sync::Arc::new"));
    }

    #[test]
    fn gen_table_with_cell_template() {
        // <Table><template slot="cell" field="name"><span>C</span></template></Table>
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("C".into())],
            slot_name: None,
            ..Default::default()
        };
        let template = Element {
            tag: "template".into(),
            attributes: vec![Attribute::Static {
                name: "field".into(),
                value: "name".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![Node::Element(span)],
            slot_name: Some("cell".to_string()),
            ..Default::default()
        };
        let table = make_element("Table", vec![], vec![Node::Element(template)]);
        let mut id = 0;
        let code = gen_table(&table, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".cell_template(\"name\","));
        assert!(code.contains("std::sync::Arc::new"));
    }

    #[test]
    fn gen_table_with_ref_uses_stable_id() {
        // <Table ref="my_table" /> → Table::new("rml_ref:my_table")
        let elem = make_element_with_directives(
            "Table",
            vec![],
            vec![Directive::Ref { name: "my_table".into(), span: Span::empty() }],
            vec![],
        );
        let mut id = 0;
        let code = gen_table(&elem, Some("my_table"), id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Table::new(\"rml_ref:my_table\")"));
        assert!(!code.contains("rml_el"));
    }

    #[test]
    fn gen_table_rejects_non_column_child() {
        // <Table><div /></Table> → 应报错（仅接受 <Column> 或 <template slot="...">）
        let div = make_element("div", vec![], vec![]);
        let table = make_element("Table", vec![], vec![Node::Element(div)]);
        let mut id = 0;
        let result = gen_table(&table, None, id, &ctx(), &mut id, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("仅支持"));
    }

    #[test]
    fn gen_table_auto_injects_notify() {
        // 所有 Table 都应自动注入 .notify(...) 回调，用于 delegate 编辑状态变更时触发重新渲染
        let elem = make_element("Table", vec![], vec![]);
        let mut id = 0;
        let code = gen_table(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".notify("), "expected .notify() in: {}", code);
        assert!(code.contains("cx.weak_entity()"), "expected cx.weak_entity() in: {}", code);
        assert!(code.contains("cx.notify()"), "expected cx.notify() in: {}", code);
    }

    #[test]
    fn gen_table_with_editable_column() {
        // <Table><Column key="name" title="Name" editable /></Table>
        let column = make_element(
            "Column",
            vec![
                Attribute::Static { name: "key".into(), value: "name".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "Name".into(), span: Span::empty() },
                Attribute::Static { name: "editable".into(), value: "".into(), span: Span::empty() },
            ],
            vec![],
        );
        let table = make_element("Table", vec![], vec![Node::Element(column)]);
        let mut id = 0;
        let code = gen_table(&table, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".editable()"), "expected .editable() in: {}", code);
    }

    #[test]
    fn gen_table_with_on_cell_edit_event() {
        // <Table on-cell-edit={handle_edit} /> → .on_cell_edit(Rc::new(...))
        let elem = make_element(
            "Table",
            vec![Attribute::Event {
                name: "on_cell_edit".into(),
                handler: crate::parser::ast::EventHandler::Ident("handle_edit".into()),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_table(&elem, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".on_cell_edit("), "expected .on_cell_edit() in: {}", code);
        assert!(code.contains("handle_edit"), "expected handle_edit method name in: {}", code);
        assert!(code.contains("cx.weak_entity()"), "expected cx.weak_entity() in: {}", code);
    }

    #[test]
    fn gen_table_with_mixed_children() {
        // <Table columns={cols}><Column key="x" title="X" /><template slot="footer"><span>F</span></template></Table>
        let column = make_element(
            "Column",
            vec![
                Attribute::Static { name: "key".into(), value: "x".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "X".into(), span: Span::empty() },
            ],
            vec![],
        );
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("F".into())],
            slot_name: None,
            ..Default::default()
        };
        let template = Element {
            tag: "template".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Element(span)],
            slot_name: Some("footer".to_string()),
            ..Default::default()
        };
        let table = make_element(
            "Table",
            vec![Attribute::Bind { name: "columns".into(), expr: "cols".into(), span: Span::empty() }],
            vec![Node::Element(column), Node::Element(template)],
        );
        let mut id = 0;
        let code = gen_table(&table, None, id, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".columns(self.cols.clone())"));
        assert!(code.contains(".column("));
        assert!(code.contains(".footer_template("));
    }

}

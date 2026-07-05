//! 插槽模板 codegen —— `<template slot="header/cell/footer">` 转译为 Arc 闭包。
//!
//! 与 `user_component.rs` 的 SlotRenderer（`Box<dyn Fn(window, cx)>`）不同，
//! Table 模板闭包带额外参数（CellTemplate 是 5 参闭包），因此需独立处理。
//!
//! ## 生成目标
//!
//! `<template slot="header">...</template>` →
//! ```text
//! .header_template(std::sync::Arc::new(move |col_idx: usize,
//!     column: &rml_ui::TableColumn, cx: &mut gpui::App| -> gpui::AnyElement {
//!     (CONTENT).into_any_element()
//! }))
//! ```
//!
//! `<template slot="cell" field="key">...</template>` →
//! ```text
//! .cell_template("key", std::sync::Arc::new(move |row_idx: usize, col_idx: usize,
//!     row_data: &rml_ui::TableRow, column: &rml_ui::TableColumn,
//!     cx: &mut gpui::App| -> gpui::AnyElement {
//!     (CONTENT).into_any_element()
//! }))
//! ```
//!
//! `<template slot="footer">...</template>` →
//! ```text
//! .footer_template(std::sync::Arc::new(move |cx: &mut gpui::App| -> gpui::AnyElement {
//!     (CONTENT).into_any_element()
//! }))
//! ```
//!
//! ## Scoped Slot 参数访问
//!
//! 模板内容可引用闭包参数（`col_idx`、`row_data` 等），通过将参数名注入 `loop_vars`
//! 实现：表达式 `{row_data.id}` 生成 `row_data.id` 而非 `self.row_data.id`。
//! `self.field` 仍可用于访问 ViewModel 字段（闭包 `move` 捕获 `self` 引用）。

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};

/// 各 slot 类型的闭包参数名（注入 loop_vars，使模板内容可引用）
fn slot_params(slot: &str) -> &'static [&'static str] {
    match slot {
        "header" => &["col_idx", "column", "cx"],
        "cell" => &["row_idx", "col_idx", "row_data", "column", "cx"],
        "footer" => &["cx"],
        _ => &[],
    }
}

/// 为 `<template slot="header/cell/footer">` 子节点生成 setter 调用
///
/// 返回完整的 `.header_template(...)` / `.cell_template(...)` / `.footer_template(...)`
/// setter 调用字符串。
pub fn gen_template(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let slot_name = elem.slot_name.as_deref().ok_or_else(|| CodegenError {
        message: "<template> 缺少 slot 属性".to_string(),
    })?;

    // 将闭包参数注入 loop_vars，使模板内容可引用（如 {row_data.id}）
    let mut scoped_vars: Vec<String> = loop_vars.to_vec();
    for p in slot_params(slot_name) {
        if !scoped_vars.iter().any(|v| v == *p) {
            scoped_vars.push((*p).to_string());
        }
    }

    let content = gen_template_content(&elem.children, ctx, id_counter, &scoped_vars)?;

    match slot_name {
        "header" => Ok(format!(
            ".header_template(std::sync::Arc::new(move |col_idx: usize, column: &rml_ui::TableColumn, cx: &mut gpui::App| -> gpui::AnyElement {{\n            ({content}).into_any_element()\n        }}))"
        )),
        "cell" => {
            let field = extract_field_attr(elem)?;
            Ok(format!(
                ".cell_template({field}, std::sync::Arc::new(move |row_idx: usize, col_idx: usize, row_data: &rml_ui::TableRow, column: &rml_ui::TableColumn, cx: &mut gpui::App| -> gpui::AnyElement {{\n            ({content}).into_any_element()\n        }}))"
            ))
        }
        "footer" => Ok(format!(
            ".footer_template(std::sync::Arc::new(move |cx: &mut gpui::App| -> gpui::AnyElement {{\n            ({content}).into_any_element()\n        }}))"
        )),
        _ => Err(CodegenError {
            message: format!(
                "未知 slot 名称 `{}`：<Table> 仅支持 slot=\"header\" / slot=\"cell\" / slot=\"footer\"",
                slot_name
            ),
        }),
    }
}

/// 从 `<template slot="cell" field="key">` 中提取 field 属性值
///
/// cell 模板必填 field，缺失报错；header/footer 模板不调用此函数。
fn extract_field_attr(elem: &Element) -> Result<String, CodegenError> {
    for attr in &elem.attributes {
        if let Attribute::Static { name, value, .. } = attr {
            if name == "field" {
                return Ok(format!("{:?}", value));
            }
        }
    }
    Err(CodegenError {
        message: "<template slot=\"cell\"> 缺少必填属性 `field`".to_string(),
    })
}

/// 为模板内容子节点列表生成构建代码
///
/// 复用 `user_component.rs::gen_slot_content` 模式：
/// - 空列表：返回 `gpui::Empty`
/// - 单节点：直接生成节点代码
/// - 多节点：包裹 `gpui::div().child(...).child(...)` 容器
fn gen_template_content(
    nodes: &[Node],
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    if nodes.is_empty() {
        return Ok("gpui::Empty".to_string());
    }
    if nodes.len() == 1 {
        let (code, _) = gen_node(&nodes[0], ctx, 0, id_counter, loop_vars)?;
        return Ok(code);
    }
    let mut code = String::from("gpui::div()");
    for node in nodes {
        let (node_code, is_iter) = gen_node(node, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!(".children({})", node_code));
        } else {
            code.push_str(&format!(".child({})", node_code));
        }
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Element, Node, TextSegment};
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    fn make_template(slot: &str, attrs: Vec<Attribute>, children: Vec<Node>) -> Element {
        Element {
            tag: "template".into(),
            attributes: attrs,
            directives: vec![],
            children,
            slot_name: Some(slot.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn gen_template_header() {
        // <template slot="header"><span>Header</span></template>
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("Header".into())],
            slot_name: None,
            ..Default::default()
        };
        let tpl = make_template("header", vec![], vec![Node::Element(span)]);
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.starts_with(".header_template("));
        assert!(code.contains("std::sync::Arc::new"));
        assert!(code.contains("col_idx: usize"));
        assert!(code.contains("column: &rml_ui::TableColumn"));
        assert!(code.contains("cx: &mut gpui::App"));
        assert!(code.contains(".into_any_element()"));
    }

    #[test]
    fn gen_template_cell_with_field() {
        // <template slot="cell" field="name"><span>Cell</span></template>
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("Cell".into())],
            slot_name: None,
            ..Default::default()
        };
        let tpl = make_template(
            "cell",
            vec![Attribute::Static { name: "field".into(), value: "name".into(), span: Span::empty() }],
            vec![Node::Element(span)],
        );
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.starts_with(".cell_template(\"name\","));
        assert!(code.contains("row_idx: usize"));
        assert!(code.contains("row_data: &rml_ui::TableRow"));
        assert!(code.contains("std::sync::Arc::new"));
    }

    #[test]
    fn gen_template_footer() {
        // <template slot="footer"><span>Footer</span></template>
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("Footer".into())],
            slot_name: None,
            ..Default::default()
        };
        let tpl = make_template("footer", vec![], vec![Node::Element(span)]);
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.starts_with(".footer_template("));
        assert!(code.contains("cx: &mut gpui::App"));
        assert!(!code.contains("row_idx"));
    }

    /// Scoped slot：cell 模板内容引用 row_data 参数
    #[test]
    fn gen_template_cell_scoped_slot_row_data() {
        // <template slot="cell" field="name"><span>{row_data.id}</span></template>
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::MixedText(vec![TextSegment::Interpolation {
                expr: "row_data.id".into(),
                span: Span::empty(),
            }])],
            slot_name: None,
            ..Default::default()
        };
        let tpl = make_template(
            "cell",
            vec![Attribute::Static { name: "field".into(), value: "name".into(), span: Span::empty() }],
            vec![Node::Element(span)],
        );
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        // 应生成 row_data.id（不带 self. 前缀）
        assert!(code.contains("row_data.id"), "expected row_data.id in: {}", code);
        assert!(!code.contains("self.row_data"));
    }

    /// Scoped slot：header 模板内容引用 column 参数
    #[test]
    fn gen_template_header_scoped_slot_column() {
        // <template slot="header"><span>{column.title}</span></template>
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::MixedText(vec![TextSegment::Interpolation {
                expr: "column.title".into(),
                span: Span::empty(),
            }])],
            slot_name: None,
            ..Default::default()
        };
        let tpl = make_template("header", vec![], vec![Node::Element(span)]);
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("column.title"), "expected column.title in: {}", code);
        assert!(!code.contains("self.column"));
    }

    /// Scoped slot：cell 模板内容引用 col_idx 参数
    #[test]
    fn gen_template_cell_scoped_slot_col_idx() {
        // <template slot="cell" field="name"><span>{col_idx}</span></template>
        let span = Element {
            tag: "span".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::MixedText(vec![TextSegment::Interpolation {
                expr: "col_idx".into(),
                span: Span::empty(),
            }])],
            slot_name: None,
            ..Default::default()
        };
        let tpl = make_template(
            "cell",
            vec![Attribute::Static { name: "field".into(), value: "name".into(), span: Span::empty() }],
            vec![Node::Element(span)],
        );
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        // col_idx 是简单标识符，应直接引用（不带 self. 前缀）
        assert!(code.contains("col_idx"), "expected col_idx in: {}", code);
        assert!(!code.contains("self.col_idx"));
    }

    #[test]
    fn gen_template_cell_missing_field_errors() {
        let tpl = make_template("cell", vec![], vec![]);
        let mut id = 0;
        let result = gen_template(&tpl, &ctx(), &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("field"));
    }

    #[test]
    fn gen_template_unknown_slot_errors() {
        let tpl = make_template("unknown", vec![], vec![]);
        let mut id = 0;
        let result = gen_template(&tpl, &ctx(), &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("unknown"));
    }

    #[test]
    fn gen_template_empty_children() {
        // <template slot="header"></template> → content = gpui::Empty
        let tpl = make_template("header", vec![], vec![]);
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("gpui::Empty"));
    }

    #[test]
    fn gen_template_multiple_children() {
        // <template slot="footer"><span>A</span><span>B</span></template>
        let make_span = |text: &str| -> Node {
            Node::Element(Element {
                tag: "span".into(),
                attributes: vec![],
                directives: vec![],
                children: vec![Node::Text(text.into())],
                slot_name: None,
                ..Default::default()
            })
        };
        let tpl = make_template("footer", vec![], vec![make_span("A"), make_span("B")]);
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("gpui::div()"));
        assert!(code.contains(".child("));
    }
}

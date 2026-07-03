//! 插槽模板 codegen —— `<template slot="header/cell/footer">` 转译为 Arc 闭包。
//!
//! 与 `user_component.rs` 的 SlotRenderer（`Box<dyn Fn(window, cx)>`）不同，
//! Table 模板闭包带额外参数（CellTemplate 是 5 参闭包），因此需独立处理。
//!
//! ## 生成目标
//!
//! `<template slot="header">...</template>` →
//! ```text
//! .header_template(std::sync::Arc::new(move |_col_idx: usize,
//!     _column: &rml_ui::TableColumn, _cx: &mut gpui::App| -> gpui::AnyElement {
//!     (CONTENT).into_any_element()
//! }))
//! ```
//!
//! `<template slot="cell" field="key">...</template>` →
//! ```text
//! .cell_template("key", std::sync::Arc::new(move |_row_idx: usize, _col_idx: usize,
//!     _row_data: &rml_ui::TableRow, _column: &rml_ui::TableColumn,
//!     _cx: &mut gpui::App| -> gpui::AnyElement {
//!     (CONTENT).into_any_element()
//! }))
//! ```
//!
//! `<template slot="footer">...</template>` →
//! ```text
//! .footer_template(std::sync::Arc::new(move |_cx: &mut gpui::App| -> gpui::AnyElement {
//!     (CONTENT).into_any_element()
//! }))
//! ```
//!
//! ## 限制
//!
//! 模板内容只能用静态元素 + `self.field` 绑定（闭包 move 捕获），不能引用闭包参数
//! （col_idx/row_data 等）。需要参数访问时用 TableDelegate trait。

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};

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

    let content = gen_template_content(&elem.children, ctx, id_counter, loop_vars)?;

    match slot_name {
        "header" => Ok(format!(
            ".header_template(std::sync::Arc::new(move |_col_idx: usize, _column: &rml_ui::TableColumn, _cx: &mut gpui::App| -> gpui::AnyElement {{\n            ({content}).into_any_element()\n        }})"
        )),
        "cell" => {
            let field = extract_field_attr(elem)?;
            Ok(format!(
                ".cell_template({field}, std::sync::Arc::new(move |_row_idx: usize, _col_idx: usize, _row_data: &rml_ui::TableRow, _column: &rml_ui::TableColumn, _cx: &mut gpui::App| -> gpui::AnyElement {{\n            ({content}).into_any_element()\n        }}))"
            ))
        }
        "footer" => Ok(format!(
            ".footer_template(std::sync::Arc::new(move |_cx: &mut gpui::App| -> gpui::AnyElement {{\n            ({content}).into_any_element()\n        }}))"
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
        if let Attribute::Static { name, value } = attr {
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
    use crate::parser::ast::{Element, Node};

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
        assert!(code.contains("_col_idx: usize"));
        assert!(code.contains("_column: &rml_ui::TableColumn"));
        assert!(code.contains("_cx: &mut gpui::App"));
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
            vec![Attribute::Static { name: "field".into(), value: "name".into() }],
            vec![Node::Element(span)],
        );
        let mut id = 0;
        let code = gen_template(&tpl, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.starts_with(".cell_template(\"name\","));
        assert!(code.contains("_row_idx: usize"));
        assert!(code.contains("_row_data: &rml_ui::TableRow"));
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
        assert!(code.contains("_cx: &mut gpui::App"));
        assert!(!code.contains("_row_idx"));
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

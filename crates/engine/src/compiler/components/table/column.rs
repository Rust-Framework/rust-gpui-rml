//! Column 子标签 codegen —— 生成直接构造表达式（非闭包）。
//!
//! TableColumn 是纯数据结构（非 IntoElement），因此不像 AccordionItem 那样
//! 生成闭包式 builder，而是直接生成 `TableColumn::new(key, title).<setters>` 表达式。
//! 由 `table::gen_table` 为每个 `<Column>` / `<column>` 子节点调用。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};

/// 为 `<Column key="..." title="..." />` 子节点生成直接构造表达式
///
/// 生成形如：
/// ```text
/// rml_ui::TableColumn::new("name", "Name").width(gpui::px(120.)).align(gpui::TextAlign::Center)
/// ```
pub fn gen_column(
    elem: &Element,
    ctx: &CodegenCtx,
    _id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. 提取 key 和 title（必填，静态或绑定）
    let key_expr = extract_required_arg(elem, "key", &lv, &computed)?;
    let title_expr = extract_required_arg(elem, "title", &lv, &computed)?;

    // 2. 构造器
    let mut code = format!("rml_ui::TableColumn::new({}, {})", key_expr, title_expr);

    // 3. 其余属性 → setter 链（委托 table setters → 公共 setter）
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if name == "key" || name == "title" {
                    continue;
                }
                if let Some(s) = super::setters::static_setter(name, value, "Column") {
                    code.push_str(&s);
                } else if let Some(s) =
                    super::super::component::component_static_setter(name, value, "Column")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if name == "key" || name == "title" {
                    continue;
                }
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "Column")
                {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_bind_setter(
                    name, expr, &lv, &computed, "Column",
                ) {
                    code.push_str(&s);
                }
            }
            _ => {}
        }
    }

    // 4. Column 是纯数据结构，不处理子节点。文本子节点报警告并忽略。
    for child in &elem.children {
        if let Node::Text(text) = child {
            eprintln!(
                "[rml warning] <Column> 不支持文本子节点 {:?}，已忽略",
                text
            );
        }
    }

    Ok(code)
}

/// 从元素属性中提取必填参数（key 或 title）
///
/// - 静态：`key="name"` → `"name"`（字符串字面量）
/// - 绑定：`key={field}` → `self.field.clone()`（SharedString）
/// - 缺失：报 CodegenError
fn extract_required_arg(
    elem: &Element,
    name: &str,
    loop_vars: &[&str],
    computed: &[&str],
) -> Result<String, CodegenError> {
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name: n, value, .. } if n == name => {
                return Ok(format!("{:?}", value));
            }
            Attribute::Bind { name: n, expr, .. } if n == name => {
                let rust_expr = super::super::component::component_bind_rust_expr(
                    expr, loop_vars, computed,
                );
                return Ok(format!("{}.clone()", rust_expr));
            }
            _ => {}
        }
    }
    Err(CodegenError {
        message: format!("<Column> 缺少必填属性 `{}`", name),
        span: Some(elem.span),
    })
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

    #[test]
    fn gen_column_minimal() {
        // <Column key="name" title="Name" /> → rml_ui::TableColumn::new("name", "Name")
        let elem = make_element(
            "Column",
            vec![
                Attribute::Static { name: "key".into(), value: "name".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "Name".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_column(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TableColumn::new(\"name\", \"Name\")"));
    }

    #[test]
    fn gen_column_with_width_and_align() {
        // <Column key="age" title="Age" width="100" align="center" />
        let elem = make_element(
            "Column",
            vec![
                Attribute::Static { name: "key".into(), value: "age".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "Age".into(), span: Span::empty() },
                Attribute::Static { name: "width".into(), value: "100".into(), span: Span::empty() },
                Attribute::Static { name: "align".into(), value: "center".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_column(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".width(gpui::px(100.))"));
        assert!(code.contains(".align(gpui::TextAlign::Center)"));
    }

    #[test]
    fn gen_column_lowercase_tag() {
        // <column key="x" title="X" /> 小写标签
        let elem = make_element(
            "column",
            vec![
                Attribute::Static { name: "key".into(), value: "x".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "X".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_column(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TableColumn::new(\"x\", \"X\")"));
    }

    #[test]
    fn gen_column_with_bind_key() {
        // <Column key={col_key} title="Title" /> → .new(self.col_key.clone(), "Title")
        let elem = make_element(
            "Column",
            vec![
                Attribute::Bind { name: "key".into(), expr: "col_key".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "Title".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_column(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("self.col_key.clone()"));
        assert!(code.contains("\"Title\""));
    }

    #[test]
    fn gen_column_missing_key_errors() {
        let elem = make_element(
            "Column",
            vec![Attribute::Static { name: "title".into(), value: "T".into(), span: Span::empty() }],
            vec![],
        );
        let mut id = 0;
        let result = gen_column(&elem, &ctx(), &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("key"));
    }

    #[test]
    fn gen_column_missing_title_errors() {
        let elem = make_element(
            "Column",
            vec![Attribute::Static { name: "key".into(), value: "k".into(), span: Span::empty() }],
            vec![],
        );
        let mut id = 0;
        let result = gen_column(&elem, &ctx(), &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("title"));
    }

    #[test]
    fn gen_column_with_bind_width() {
        // <Column key="x" title="X" width={col_w} /> → .width(self.col_w)
        let elem = make_element(
            "Column",
            vec![
                Attribute::Static { name: "key".into(), value: "x".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "X".into(), span: Span::empty() },
                Attribute::Bind { name: "width".into(), expr: "col_w".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_column(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".width(self.col_w)"));
    }

    #[test]
    fn gen_column_ignores_text_child() {
        let elem = make_element(
            "Column",
            vec![
                Attribute::Static { name: "key".into(), value: "x".into(), span: Span::empty() },
                Attribute::Static { name: "title".into(), value: "X".into(), span: Span::empty() },
            ],
            vec![Node::Text("ignored".into())],
        );
        let mut id = 0;
        let code = gen_column(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        // 文本子节点被忽略，不影响构造表达式
        assert!(!code.contains("ignored"));
    }
}

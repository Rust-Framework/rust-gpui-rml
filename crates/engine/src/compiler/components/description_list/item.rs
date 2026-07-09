//! 单个 `<description>` / `<DescriptionItem>` 子节点 codegen
//!
//! 生成 `rml_ui::DescriptionItem::new(label).<setters>` 直接构造表达式。
//! 由 `gen::gen_description_list` 为每个 `<description>` 子节点调用。
//!
//! ## label 与 value 的处理
//!
//! - `label` 属性（必填）→ 构造器参数 `DescriptionItem::new(label)`
//! - `value` 属性 → `.value(...)` setter
//! - 无 `value` 属性时：文本子节点 → `.value("text")`，element 子节点 → `.value(element)`
//! - `value` 属性优先于子节点

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};

/// 为 `<description>` 子节点生成 `rml_ui::DescriptionItem::new(label)...` 表达式
///
/// 生成形如：
/// ```text
/// rml_ui::DescriptionItem::new("Name").value("John").span(1)
/// rml_ui::DescriptionItem::new("Status").value(gpui::div().child(...))
/// ```
pub fn gen_description_item(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. 提取 label（必填，静态或绑定）作为构造器参数
    let label_expr = extract_required_label(elem, &lv, &computed)?;

    // 2. 构造器
    let mut code = format!("rml_ui::DescriptionItem::new({})", label_expr);

    // 3. 其余属性 → setter 链（委托 description_list setters → 公共 setter）
    let mut value_set_by_attr = false;
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if name == "label" {
                    continue;
                }
                if let Some(s) = super::setters::static_setter(name, value, "DescriptionItem") {
                    code.push_str(&s);
                    if name == "value" {
                        value_set_by_attr = true;
                    }
                } else if let Some(s) = super::super::component::component_static_setter(
                    name,
                    value,
                    "DescriptionItem",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if name == "label" {
                    continue;
                }
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "DescriptionItem")
                {
                    code.push_str(&s);
                    if name == "value" {
                        value_set_by_attr = true;
                    }
                } else if let Some(s) = super::super::component::component_bind_setter(
                    name,
                    expr,
                    &lv,
                    &computed,
                    "DescriptionItem",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = super::super::component::component_event_setter(
                    name,
                    handler,
                    "DescriptionItem",
                ) {
                    code.push_str(&s);
                }
            }
        }
    }

    // 4. 子节点 → value（仅当无 value 属性时）
    if !value_set_by_attr {
        let value_from_children = extract_value_from_children(elem, ctx, id_counter, loop_vars)?;
        if let Some(val_code) = value_from_children {
            code.push_str(&format!(".value({})", val_code));
        }
    }

    Ok(code)
}

/// 从元素属性中提取必填的 label 参数
///
/// - 静态：`label="Name"` → `"Name"`（字符串字面量）
/// - 绑定：`label={field}` → `self.field.clone()`（DescriptionText: From<SharedString> 需要 owned）
/// - 缺失：报 CodegenError
fn extract_required_label(
    elem: &Element,
    loop_vars: &[&str],
    computed: &[&str],
) -> Result<String, CodegenError> {
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == "label" => {
                return Ok(format!("{:?}", value));
            }
            Attribute::Bind { name, expr, .. } if name == "label" => {
                let rust_expr =
                    super::super::component::component_bind_rust_expr(expr, loop_vars, computed);
                return Ok(format!("{}.clone()", rust_expr));
            }
            _ => {}
        }
    }
    Err(CodegenError {
        message: "<description> 缺少必填属性 `label`".to_string(),
        span: Some(elem.span),
    })
}

/// 从子节点提取 value 表达式（无 value 属性时）
///
/// - 文本子节点 → `"text"`（第一个文本节点）
/// - 单个 element 子节点 → element 构造代码
/// - 多个 element 子节点 → `gpui::div().child(e1).child(e2)...`
/// - 无子节点 → None
fn extract_value_from_children(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<Option<String>, CodegenError> {
    let mut text_parts: Vec<&str> = Vec::new();
    let mut element_codes: Vec<String> = Vec::new();

    for child in &elem.children {
        match child {
            Node::Text(text) => text_parts.push(text.as_str()),
            Node::Element(_) => {
                let (child_code, _is_iter) =
                    gen_node(child, ctx, 0, id_counter, loop_vars)?;
                element_codes.push(child_code);
            }
            _ => {}
        }
    }

    // 优先级：文本子节点 > element 子节点
    if !text_parts.is_empty() {
        let combined = text_parts.join("");
        return Ok(Some(format!("{:?}", combined)));
    }

    if element_codes.is_empty() {
        return Ok(None);
    }

    if element_codes.len() == 1 {
        return Ok(Some(element_codes.into_iter().next().unwrap()));
    }

    // 多个 element 子节点包装为 div
    let mut wrapper = String::from("gpui::div()");
    for code in element_codes {
        wrapper.push_str(&format!(".child({})", code));
    }
    Ok(Some(wrapper))
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
    fn gen_item_minimal() {
        // <description label="Name" value="John" />
        let elem = make_element(
            "description",
            vec![
                Attribute::Static { name: "label".into(), value: "Name".into(), span: Span::empty() },
                Attribute::Static { name: "value".into(), value: "John".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::DescriptionItem::new(\"Name\")"));
        assert!(code.contains(".value(\"John\")"));
    }

    #[test]
    fn gen_item_pascalcase_tag() {
        // <DescriptionItem label="Name" value="John" />
        let elem = make_element(
            "DescriptionItem",
            vec![
                Attribute::Static { name: "label".into(), value: "Name".into(), span: Span::empty() },
                Attribute::Static { name: "value".into(), value: "John".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::DescriptionItem::new(\"Name\")"));
    }

    #[test]
    fn gen_item_missing_label_errors() {
        let elem = make_element(
            "description",
            vec![Attribute::Static { name: "value".into(), value: "John".into(), span: Span::empty() }],
            vec![],
        );
        let mut id = 0;
        let result = gen_description_item(&elem, &ctx(), &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("label"));
    }

    #[test]
    fn gen_item_with_span() {
        // <description label="Name" value="John" span="2" />
        let elem = make_element(
            "description",
            vec![
                Attribute::Static { name: "label".into(), value: "Name".into(), span: Span::empty() },
                Attribute::Static { name: "value".into(), value: "John".into(), span: Span::empty() },
                Attribute::Static { name: "span".into(), value: "2".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".span(2)"));
    }

    #[test]
    fn gen_item_with_bind_label() {
        // <description label={item.label} value="John" />
        let elem = make_element(
            "description",
            vec![
                Attribute::Bind { name: "label".into(), expr: "item.label".into(), span: Span::empty() },
                Attribute::Static { name: "value".into(), value: "John".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("self.item.label.clone()"));
    }

    #[test]
    fn gen_item_with_bind_value() {
        // <description label="Name" value={user.name} />
        let elem = make_element(
            "description",
            vec![
                Attribute::Static { name: "label".into(), value: "Name".into(), span: Span::empty() },
                Attribute::Bind { name: "value".into(), expr: "user.name".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".value(self.user.name.clone())"));
    }

    #[test]
    fn gen_item_with_bind_span() {
        // <description label="Name" value="John" span={item_span} />
        let elem = make_element(
            "description",
            vec![
                Attribute::Static { name: "label".into(), value: "Name".into(), span: Span::empty() },
                Attribute::Static { name: "value".into(), value: "John".into(), span: Span::empty() },
                Attribute::Bind { name: "span".into(), expr: "item_span".into(), span: Span::empty() },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".span(self.item_span)"));
    }

    #[test]
    fn gen_item_text_child_as_value() {
        // <description label="Name">John Doe</description>
        let elem = make_element(
            "description",
            vec![Attribute::Static { name: "label".into(), value: "Name".into(), span: Span::empty() }],
            vec![Node::Text("John Doe".into())],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".value(\"John Doe\")"));
    }

    #[test]
    fn gen_item_value_attr_overrides_text_child() {
        // <description label="Name" value="Attr">Child</description>
        // value 属性优先，文本子节点被忽略
        let elem = make_element(
            "description",
            vec![
                Attribute::Static { name: "label".into(), value: "Name".into(), span: Span::empty() },
                Attribute::Static { name: "value".into(), value: "Attr".into(), span: Span::empty() },
            ],
            vec![Node::Text("Child".into())],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".value(\"Attr\")"));
        assert!(!code.contains(".value(\"Child\")"));
    }

    #[test]
    fn gen_item_element_child_as_value() {
        // <description label="Status"><Badge success>Active</Badge></description>
        let badge = make_element("Badge", vec![], vec![Node::Text("Active".into())]);
        let elem = make_element(
            "description",
            vec![Attribute::Static { name: "label".into(), value: "Status".into(), span: Span::empty() }],
            vec![Node::Element(badge)],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".value("));
        // element 子节点作为 value
        assert!(code.contains("Badge"));
    }

    #[test]
    fn gen_item_no_value_no_children() {
        // <description label="Name" /> — 无 value 也无子节点
        let elem = make_element(
            "description",
            vec![Attribute::Static { name: "label".into(), value: "Name".into(), span: Span::empty() }],
            vec![],
        );
        let mut id = 0;
        let code = gen_description_item(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::DescriptionItem::new(\"Name\")"));
        // 不应包含 .value(
        assert!(!code.contains(".value("));
    }
}

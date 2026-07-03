//! DescriptionList 容器 codegen —— 构造 + 属性 + 子节点 `.child()`/`.separator()` 注入。
//!
//! 将 `<descriptions><description label="A" value="B" /><separator /></descriptions>` 转译为
//! `rml_ui::DescriptionList::new().child(rml_ui::DescriptionItem::new("A").value("B")).separator()`。
//!
//! 与 TabBar 的关键差异：
//! - 构造器无 ElementId（`DescriptionList::new()`），ref 指令静默忽略
//! - 子节点有两种类型：`<description>` → `.child(...)`，`<separator>` → `.separator()`

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 生成 DescriptionList 构造代码（构造 + 属性 + 子节点注入）
///
/// 由 `component::gen_component` 在 `StatelessWithItems` 分支按 canonical_tag == "DescriptionList" 调用。
///
/// ref 指令静默忽略（DescriptionList::new() 不接受 ElementId）。
pub fn gen_description_list(
    elem: &Element,
    _ref_name: Option<&str>,
    _id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    // 1. 构造器（无 ElementId，ref 指令静默忽略）
    let mut code = String::from("rml_ui::DescriptionList::new()");

    // 2. 属性 → setter（先调 description_list 专用 setter，未命中回退到公共 setter）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } => {
                if let Some(s) = super::setters::static_setter(name, value, "DescriptionList") {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_static_setter(
                    name,
                    value,
                    "DescriptionList",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "DescriptionList")
                {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_bind_setter(
                    name,
                    expr,
                    &lv,
                    &computed,
                    "DescriptionList",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler } => {
                if let Some(s) = super::super::component::component_event_setter(
                    name,
                    handler,
                    "DescriptionList",
                ) {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点 → .child(DescriptionItem::new()...) 或 .separator()
    for child in &elem.children {
        match child {
            Node::Element(child_elem) => {
                let canonical = tags::canonical_tag(&child_elem.tag);
                match canonical.as_str() {
                    "DescriptionItem" => {
                        let item_code = super::item::gen_description_item(
                            child_elem,
                            ctx,
                            id_counter,
                            loop_vars,
                        )?;
                        code.push_str(&format!("\n            .child({})", item_code));
                    }
                    "DescriptionSeparator" => {
                        code.push_str("\n            .separator()");
                    }
                    _ => {
                        return Err(CodegenError {
                            message: format!(
                                "<descriptions> 仅支持 <description> 或 <separator> 子节点，得到 <{}>",
                                child_elem.tag
                            ),
                        });
                    }
                }
            }
            Node::Text(text) => {
                eprintln!(
                    "[rml warning] <descriptions> 不支持文本子节点 {:?}，已忽略",
                    text
                );
            }
            _ => {}
        }
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Directive, Element, Node};

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
    fn gen_description_list_minimal() {
        // <descriptions /> → rml_ui::DescriptionList::new()
        let elem = make_element("descriptions", vec![], vec![]);
        let mut id = 0;
        let code =
            gen_description_list(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::DescriptionList::new()"));
    }

    #[test]
    fn gen_description_list_pascalcase_tag() {
        // <DescriptionList /> 也应正常工作
        let elem = make_element("DescriptionList", vec![], vec![]);
        let mut id = 0;
        let code =
            gen_description_list(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::DescriptionList::new()"));
    }

    #[test]
    fn gen_description_list_with_static_props() {
        // <descriptions vertical columns="2" bordered="false" />
        let elem = make_element(
            "descriptions",
            vec![
                Attribute::Static { name: "vertical".into(), value: "".into() },
                Attribute::Static { name: "columns".into(), value: "2".into() },
                Attribute::Static { name: "bordered".into(), value: "false".into() },
            ],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_description_list(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".layout(gpui::Axis::Vertical)"));
        assert!(code.contains(".columns(2)"));
        assert!(code.contains(".bordered(false)"));
    }

    #[test]
    fn gen_description_list_with_label_width() {
        // <descriptions label_width="200" />
        let elem = make_element(
            "descriptions",
            vec![Attribute::Static {
                name: "label_width".into(),
                value: "200".into(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_description_list(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".label_width(gpui::px(200.))"));
    }

    #[test]
    fn gen_description_list_with_description_child() {
        // <descriptions><description label="Name" value="John" /></descriptions>
        let desc = make_element(
            "description",
            vec![
                Attribute::Static { name: "label".into(), value: "Name".into() },
                Attribute::Static { name: "value".into(), value: "John".into() },
            ],
            vec![],
        );
        let list = make_element("descriptions", vec![], vec![Node::Element(desc)]);
        let mut id = 0;
        let code = gen_description_list(&list, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".child("));
        assert!(code.contains("rml_ui::DescriptionItem::new(\"Name\")"));
        assert!(code.contains(".value(\"John\")"));
    }

    #[test]
    fn gen_description_list_with_separator_child() {
        // <descriptions><separator /></descriptions>
        let sep = make_element("separator", vec![], vec![]);
        let list = make_element("descriptions", vec![], vec![Node::Element(sep)]);
        let mut id = 0;
        let code = gen_description_list(&list, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".separator()"));
    }

    #[test]
    fn gen_description_list_with_pascalcase_separator() {
        // <DescriptionList><DescriptionSeparator /></DescriptionList>
        let sep = make_element("DescriptionSeparator", vec![], vec![]);
        let list = make_element("DescriptionList", vec![], vec![Node::Element(sep)]);
        let mut id = 0;
        let code = gen_description_list(&list, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".separator()"));
    }

    #[test]
    fn gen_description_list_mixed_children() {
        // <descriptions>
        //   <description label="A" value="1" />
        //   <separator />
        //   <description label="B" value="2" />
        // </descriptions>
        let desc1 = make_element(
            "description",
            vec![
                Attribute::Static { name: "label".into(), value: "A".into() },
                Attribute::Static { name: "value".into(), value: "1".into() },
            ],
            vec![],
        );
        let sep = make_element("separator", vec![], vec![]);
        let desc2 = make_element(
            "description",
            vec![
                Attribute::Static { name: "label".into(), value: "B".into() },
                Attribute::Static { name: "value".into(), value: "2".into() },
            ],
            vec![],
        );
        let list = make_element(
            "descriptions",
            vec![],
            vec![
                Node::Element(desc1),
                Node::Element(sep),
                Node::Element(desc2),
            ],
        );
        let mut id = 0;
        let code = gen_description_list(&list, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        // 应有两个 .child( 和一个 .separator()
        assert_eq!(code.matches(".child(").count(), 2);
        assert_eq!(code.matches(".separator()").count(), 1);
        assert!(code.contains("\"A\""));
        assert!(code.contains("\"B\""));
    }

    #[test]
    fn gen_description_list_ref_ignored() {
        // <descriptions ref="my_list" /> — ref 静默忽略，不生成 id 参数
        let elem = make_element_with_directives(
            "descriptions",
            vec![],
            vec![Directive::Ref("my_list".into())],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_description_list(&elem, Some("my_list"), id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::DescriptionList::new()"));
        // 不应包含 ref id
        assert!(!code.contains("rml_ref"));
    }

    #[test]
    fn gen_description_list_rejects_invalid_child() {
        // <descriptions><div /></descriptions> → 应报错
        let div = make_element("div", vec![], vec![]);
        let list = make_element("descriptions", vec![], vec![Node::Element(div)]);
        let mut id = 0;
        let result = gen_description_list(&list, None, id, &ctx(), &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("仅支持 <description> 或 <separator> 子节点"));
    }

    #[test]
    fn gen_description_list_with_sizable() {
        // <descriptions size="small" vertical> → .with_size(Size::Small) + .layout(Vertical)
        let elem = make_element(
            "descriptions",
            vec![
                Attribute::Static { name: "size".into(), value: "small".into() },
                Attribute::Static { name: "vertical".into(), value: "".into() },
            ],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_description_list(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".with_size(rml_ui::Size::Small)"));
        assert!(code.contains(".layout(gpui::Axis::Vertical)"));
    }

    #[test]
    fn gen_description_list_with_bind_props() {
        // <descriptions columns={col_count} bordered={show_border} />
        let elem = make_element(
            "descriptions",
            vec![
                Attribute::Bind { name: "columns".into(), expr: "col_count".into() },
                Attribute::Bind { name: "bordered".into(), expr: "show_border".into() },
            ],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_description_list(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".columns(self.col_count)"));
        assert!(code.contains(".bordered(self.show_border)"));
    }

    /// 端到端验证：通过 gen_component 入口调用
    #[test]
    fn gen_description_list_via_gen_component_dispatch() {
        use crate::compiler::component::gen_component;
        let elem = make_element("descriptions", vec![], vec![]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::DescriptionList::new()"));
    }

    /// PascalCase 标签通过 gen_component 入口调度
    #[test]
    fn gen_description_list_pascalcase_via_gen_component() {
        use crate::compiler::component::gen_component;
        let elem = make_element("DescriptionList", vec![], vec![]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::DescriptionList::new()"));
    }
}

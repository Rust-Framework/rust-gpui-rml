//! Accordion 容器 codegen —— 构造 + 属性 + 子节点 .item() 注入。
//!
//! 将 `<Accordion><AccordionItem ...>...</AccordionItem></Accordion>` 转译为
//! `rml_ui::Accordion::new(id).multiple(true).item(|__rml_item| __rml_item.title(...).child(...))`。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, EventHandler, Node};
use crate::tags;

/// 生成 Accordion 构造代码（构造 + 属性 + 子节点 .item() 注入）
///
/// 由 `AccordionTranslator` 调用，
/// 整个 Accordion codegen 流程自包含于此。
pub fn gen_accordion(
    elem: &Element,
    ref_name: Option<&str>,
    id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    // 1. 构造器
    let mut code = if let Some(name) = ref_name {
        format!("rml_ui::Accordion::new({:?})", format!("rml_ref:{}", name))
    } else {
        format!("rml_ui::Accordion::new((\"rml_el\", {}usize))", id_val)
    };

    // 2. 属性 → setter（先调 accordion 专用 setter，未命中回退到公共 setter 处理 Sizable 等通用属性）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 预扫描：受控模式 open-ixs 绑定 + 用户自定义 on-toggle-click
    // 注意：RML 属性名在 parser 中已从 kebab-case 规范化为 snake_case
    let open_ixs_expr = elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Bind { name, expr, .. } if name == "open_ixs" => Some(
            crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed),
        ),
        _ => None,
    });
    let user_on_toggle = elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Event { name, handler, .. } if name == "on_toggle_click" => {
            Some(handler.clone())
        }
        _ => None,
    });

    for attr in &elem.attributes {
        match attr {
            // open-ixs 由受控模式统一处理，不生成普通 setter
            Attribute::Bind { name, .. } if name == "open_ixs" => continue,
            // on-toggle-click 在存在 open-ixs 时由受控模式生成组合回调
            Attribute::Event { name, handler, .. } if name == "on_toggle_click" => {
                if open_ixs_expr.is_none() {
                    if let Some(s) = super::setters::event_setter(name, handler, "Accordion") {
                        code.push_str(&s);
                    }
                }
                continue;
            }
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value, "Accordion") {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Accordion")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "Accordion")
                {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "Accordion",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = super::setters::event_setter(name, handler, "Accordion") {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Accordion")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点 → .item(|__rml_item| ...) 闭包
    //    若启用受控模式 open-ixs，自动为未显式指定 open 的 item 追加 .open(self.<field>.contains(&ix))
    let mut item_index: usize = 0;
    for child in &elem.children {
        match child {
            Node::Element(child_elem) if tags::is_item_builder_tag(&child_elem.tag) => {
                let mut item_code =
                    super::item::gen_item_builder(child_elem, ctx, id_counter, loop_vars)?;
                if let Some(ref expr) = open_ixs_expr {
                    let has_explicit_open = child_elem.attributes.iter().any(|attr| match attr {
                        Attribute::Static { name, .. } | Attribute::Bind { name, .. } => {
                            name == "open"
                        }
                        _ => false,
                    });
                    if !has_explicit_open {
                        item_code.push_str(&format!(
                            ".open({}.contains(&{}usize))",
                            expr, item_index
                        ));
                    }
                }
                code.push_str(&format!("\n            .item({})", item_code));
                item_index += 1;
            }
            Node::Text(text) => {
                eprintln!(
                    "[rml warning] <Accordion> 不支持文本子节点 {:?}，已忽略",
                    text
                );
            }
            Node::Element(child_elem) => {
                return Err(CodegenError {
                    message: format!(
                        "<accordion> 仅支持 <item> 或 <AccordionItem> 子节点，得到 <{}>",
                        child_elem.tag
                    ),
                    span: Some(elem.span),
                });
            }
            _ => {}
        }
    }

    // 4. 受控模式：生成 on-toggle-click 回调，同步 open-ixs 字段并可选调用用户回调
    if let Some(ref expr) = open_ixs_expr {
        // expr 可能是 "self.field" 或 "__rml_self_ref.field"（slot 闭包内），
        // 反向同步在 cx.listener 闭包内，应使用 "this.field"，需剥离前缀只保留字段名。
        let field_name = expr
            .strip_prefix("self.")
            .or_else(|| expr.strip_prefix("__rml_self_ref."))
            .unwrap_or(expr)
            .to_string();
        let callback = match user_on_toggle {
            Some(ref handler) => {
                let method = match handler {
                    EventHandler::Ident(m) | EventHandler::MethodName(m) => m.clone(),
                    EventHandler::WithArgs(m, _) => m.clone(),
                };
                format!(
                    ".on_toggle_click(cx.listener(move |this, open_ixs: &[usize], _window, cx| {{\n                    \
                     this.{} = open_ixs.to_vec();\n                    \
                     this.{}(open_ixs, cx);\n                }}))",
                    field_name, method
                )
            }
            None => format!(
                ".on_toggle_click(cx.listener(move |this, open_ixs: &[usize], _window, cx| {{\n                    \
                     this.{} = open_ixs.to_vec();\n                    \
                     cx.notify();\n                }}))",
                field_name
            ),
        };
        code.push_str(&callback);
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Directive, Element, EventHandler, Node};
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
    fn gen_accordion_minimal() {
        // <Accordion /> → rml_ui::Accordion::new(("rml_el", 0usize))
        let elem = make_element("Accordion", vec![], vec![]);
        let mut id = 0;
        let code = gen_accordion(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Accordion::new"));
        assert!(code.contains("\"rml_el\""));
    }

    #[test]
    fn gen_accordion_with_static_props() {
        // <Accordion multiple="" bordered="" /> → .multiple(true).bordered(true)
        let elem = make_element(
            "Accordion",
            vec![
                Attribute::Static {
                    name: "multiple".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "bordered".into(),
                    value: "true".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_accordion(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".multiple(true)"));
        assert!(code.contains(".bordered(true)"));
    }

    #[test]
    fn gen_accordion_with_item() {
        // <Accordion><AccordionItem title="Section 1"><p>Content</p></AccordionItem></Accordion>
        let item = make_element(
            "AccordionItem",
            vec![Attribute::Static {
                name: "title".into(),
                value: "Section 1".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("Content".into())],
        );
        let accordion = make_element("Accordion", vec![], vec![Node::Element(item)]);
        let mut id = 0;
        let code =
            gen_accordion(&accordion, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".item("));
        assert!(code.contains("|__rml_item: rml_ui::AccordionItem|"));
        assert!(code.contains(".title(\"Section 1\")"));
        assert!(code.contains(".child(\"Content\")"));
    }

    #[test]
    fn gen_accordion_with_open_and_icon() {
        // <AccordionItem open="" icon="Settings">...</AccordionItem>
        let item = make_element(
            "AccordionItem",
            vec![
                Attribute::Static {
                    name: "open".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "icon".into(),
                    value: "Settings".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let accordion = make_element("Accordion", vec![], vec![Node::Element(item)]);
        let mut id = 0;
        let code =
            gen_accordion(&accordion, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".open(true)"));
        assert!(code.contains(".icon(rml_ui::IconName::Settings)"));
    }

    #[test]
    fn gen_accordion_with_on_toggle_click() {
        // <Accordion on_toggle_click={on_toggle} />
        let elem = make_element(
            "Accordion",
            vec![Attribute::Event {
                name: "on_toggle_click".into(),
                handler: EventHandler::Ident("on_toggle".into()),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_accordion(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".on_toggle_click("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("open_ixs: &[usize]"));
        assert!(code.contains("this.on_toggle"));
    }

    #[test]
    fn gen_accordion_with_ref_uses_stable_id() {
        // <Accordion ref="my_accordion" /> → Accordion::new("rml_ref:my_accordion")
        let elem = make_element_with_directives(
            "Accordion",
            vec![],
            vec![Directive::Ref { name: "my_accordion".into(), span: Span::empty() }],
            vec![],
        );
        let mut id = 0;
        let code = gen_accordion(
            &elem,
            Some("my_accordion"),
            id,
            &ctx(),
            &mut id,
            &Vec::new(),
        )
        .unwrap();
        assert!(code.contains("rml_ui::Accordion::new(\"rml_ref:my_accordion\")"));
        assert!(!code.contains("rml_el"));
    }

    #[test]
    fn gen_accordion_rejects_non_item_child() {
        // <Accordion><div /></Accordion> → 应报错（仅接受 <AccordionItem>）
        let div = make_element("div", vec![], vec![]);
        let accordion = make_element("Accordion", vec![], vec![Node::Element(div)]);
        let mut id = 0;
        let result = gen_accordion(&accordion, None, id, &ctx(), &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("仅支持 <item>"));
    }

    #[test]
    fn gen_accordion_with_sizable() {
        // <Accordion size="small" bordered="" /> → .with_size(Size::Small) + .bordered(true)
        // size 走通用 Sizable setter，bordered 走 accordion 专用 setter
        let elem = make_element(
            "Accordion",
            vec![
                Attribute::Static {
                    name: "size".into(),
                    value: "small".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "bordered".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_accordion(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".with_size(rml_ui::Size::Small)"));
        assert!(code.contains(".bordered(true)"));
    }

    /// <item> 短标签作为 <accordion> 子节点
    #[test]
    fn gen_accordion_with_item_short_form() {
        let item = make_element(
            "item",
            vec![Attribute::Static {
                name: "title".into(),
                value: "Section 1".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("Content".into())],
        );
        let accordion = make_element("accordion", vec![], vec![Node::Element(item)]);
        let mut id = 0;
        let code =
            gen_accordion(&accordion, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".item("));
        assert!(code.contains("|__rml_item: rml_ui::AccordionItem|"));
        assert!(code.contains(".title(\"Section 1\")"));
        assert!(code.contains(".child(\"Content\")"));
    }
}

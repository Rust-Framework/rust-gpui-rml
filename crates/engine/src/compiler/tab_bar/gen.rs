//! TabBar 容器 codegen —— 构造 + 属性 + 子节点 `.child(Tab::new()...)` 注入。
//!
//! 将 `<TabBar><Tab label="A" /><Tab><Icon /><span>A</span></Tab></TabBar>` 转译为
//! `rml_ui::TabBar::new(id).underline().selected_index(0).child(rml_ui::Tab::new().label("A")).child(rml_ui::Tab::new().child(...))`。
//!
//! 与 Accordion 的关键差异：子节点通过 `.child(Tab::new()...)` 直接注入，
//! 而非 `.item(|__rml_item| __rml_item...)` 闭包。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 生成 TabBar 构造代码（构造 + 属性 + 子节点 .child(Tab) 注入）
///
/// 由 `component::gen_component` 在 `StatelessWithItems` 分支按 tag == "TabBar" 调用。
pub fn gen_tab_bar(
    elem: &Element,
    ref_name: Option<&str>,
    id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    // 1. 构造器
    let mut code = if let Some(name) = ref_name {
        format!("rml_ui::TabBar::new({:?})", format!("rml_ref:{}", name))
    } else {
        format!("rml_ui::TabBar::new((\"rml_el\", {}usize))", id_val)
    };

    // 2. 属性 → setter（先调 tab_bar 专用 setter，未命中回退到公共 setter 处理 Sizable 等通用属性）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value, "TabBar") {
                    code.push_str(&s);
                } else if let Some(s) =
                    super::super::component::component_static_setter(name, value, "TabBar")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "TabBar")
                {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_bind_setter(
                    name, expr, &lv, &computed, "TabBar",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = super::setters::event_setter(name, handler, "TabBar") {
                    code.push_str(&s);
                } else if let Some(s) =
                    super::super::component::component_event_setter(name, handler, "TabBar")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点 → .child(Tab/TabItem) 直接构造
    for child in &elem.children {
        match child {
            Node::Element(child_elem) if tags::is_item_builder_tag(&child_elem.tag) => {
                let canonical = tags::canonical_tag(&child_elem.tag);
                if canonical == "TabItem" {
                    let (item_code, is_iter) =
                        super::tab_item::gen_tab_item_child(child_elem, ctx, id_counter, loop_vars)?;
                    if is_iter {
                        code.push_str(&format!("\n            .children({})", item_code));
                    } else {
                        code.push_str(&format!("\n            .child({})", item_code));
                    }
                } else {
                    let (tab_code, is_iter) =
                        super::tab::gen_tab_child(child_elem, ctx, id_counter, loop_vars)?;
                    if is_iter {
                        code.push_str(&format!("\n            .children({})", tab_code));
                    } else {
                        code.push_str(&format!("\n            .child({})", tab_code));
                    }
                }
            }
            Node::Text(text) => {
                eprintln!(
                    "[rml warning] <TabBar> 不支持文本子节点 {:?}，已忽略",
                    text
                );
            }
            Node::Element(child_elem) => {
                return Err(CodegenError {
                    message: format!(
                        "<TabBar> 仅支持 <Tab>/<TabItem> 子节点，得到 <{}>",
                        child_elem.tag
                    ),
                });
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
    use crate::parser::ast::{Attribute, Directive, EachClause, Element, EventHandler, Node};
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
    fn gen_tab_bar_minimal() {
        // <TabBar /> → rml_ui::TabBar::new(("rml_el", 0usize))
        let elem = make_element("TabBar", vec![], vec![]);
        let mut id = 0;
        let code = gen_tab_bar(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TabBar::new"));
        assert!(code.contains("\"rml_el\""));
    }

    #[test]
    fn gen_tab_bar_with_static_props() {
        // <TabBar underline menu="true" /> → .underline().menu(true)
        let elem = make_element(
            "TabBar",
            vec![
                Attribute::Static {
                    name: "underline".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "menu".into(),
                    value: "true".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_tab_bar(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".underline()"));
        assert!(code.contains(".menu(true)"));
    }

    #[test]
    fn gen_tab_bar_with_tab_child() {
        // <TabBar><Tab label="Account" /></TabBar>
        let tab = make_element(
            "Tab",
            vec![Attribute::Static {
                name: "label".into(),
                value: "Account".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let bar = make_element("TabBar", vec![], vec![Node::Element(tab)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".child("));
        assert!(code.contains("rml_ui::Tab::new()"));
        assert!(code.contains(".label(\"Account\")"));
    }

    #[test]
    fn gen_tab_bar_with_tab_text_child() {
        // <TabBar><Tab>Account</Tab></TabBar> → .child(rml_ui::Tab::new().label("Account"))
        let tab = make_element("Tab", vec![], vec![Node::Text("Account".into())]);
        let bar = make_element("TabBar", vec![], vec![Node::Element(tab)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Tab::new()"));
        assert!(code.contains(".label(\"Account\")"));
    }

    #[test]
    fn gen_tab_bar_with_tab_template_child() {
        // <TabBar><Tab><span>Account</span></Tab></TabBar> → .child(rml_ui::Tab::new().child(...))
        let span = make_element("span", vec![], vec![Node::Text("Account".into())]);
        let tab = make_element("Tab", vec![], vec![Node::Element(span)]);
        let bar = make_element("TabBar", vec![], vec![Node::Element(tab)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Tab::new()"));
        assert!(code.contains(".child("));
        // 不应将文本映射为 .label() —— 因为子节点是 element，非纯文本
        assert!(!code.contains(".label(\"Account\")"));
    }

    #[test]
    fn gen_tab_bar_with_tab_icon() {
        // <TabBar><Tab icon="User" label="Account" /></TabBar>
        let tab = make_element(
            "Tab",
            vec![
                Attribute::Static {
                    name: "icon".into(),
                    value: "User".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "label".into(),
                    value: "Account".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let bar = make_element("TabBar", vec![], vec![Node::Element(tab)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".icon(rml_ui::IconName::User)"));
        assert!(code.contains(".label(\"Account\")"));
    }

    #[test]
    fn gen_tab_bar_with_on_click() {
        // <TabBar on_click={on_tab_select} /> → .on_click(cx.listener(move |this, idx: &usize, ...))
        let elem = make_element(
            "TabBar",
            vec![Attribute::Event {
                name: "on_click".into(),
                handler: EventHandler::Ident("on_tab_select".into()),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_tab_bar(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".on_click("));
        assert!(code.contains("cx.listener"));
        assert!(code.contains("idx: &usize"));
        assert!(code.contains("this.on_tab_select(*idx, cx)"));
    }

    #[test]
    fn gen_tab_bar_with_selected_index_bind() {
        // <TabBar selected_index={active_tab} />
        let elem = make_element(
            "TabBar",
            vec![Attribute::Bind {
                name: "selected_index".into(),
                expr: "active_tab".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_tab_bar(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".selected_index(self.active_tab)"));
    }

    #[test]
    fn gen_tab_bar_with_ref_uses_stable_id() {
        // <TabBar ref="my_tabs" /> → TabBar::new("rml_ref:my_tabs")
        let elem = make_element_with_directives(
            "TabBar",
            vec![],
            vec![Directive::Ref { name: "my_tabs".into(), span: Span::empty() }],
            vec![],
        );
        let mut id = 0;
        let code = gen_tab_bar(
            &elem,
            Some("my_tabs"),
            id,
            &ctx(),
            &mut id,
            &Vec::new(),
        )
        .unwrap();
        assert!(code.contains("rml_ui::TabBar::new(\"rml_ref:my_tabs\")"));
        assert!(!code.contains("rml_el"));
    }

    #[test]
    fn gen_tab_bar_rejects_non_tab_child() {
        // <TabBar><div /></TabBar> → 应报错
        let div = make_element("div", vec![], vec![]);
        let bar = make_element("TabBar", vec![], vec![Node::Element(div)]);
        let mut id = 0;
        let result = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("仅支持 <Tab>/<TabItem> 子节点"));
    }

    #[test]
    fn gen_tab_bar_with_sizable() {
        // <TabBar size="small" underline> → .with_size(Size::Small) + .underline()
        let elem = make_element(
            "TabBar",
            vec![
                Attribute::Static {
                    name: "size".into(),
                    value: "small".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "underline".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_tab_bar(&elem, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".with_size(rml_ui::Size::Small)"));
        assert!(code.contains(".underline()"));
    }

    #[test]
    fn gen_tab_bar_with_multiple_tabs() {
        // <TabBar><Tab label="A" /><Tab label="B" /></TabBar> → 两个 .child(...)
        let tab1 = make_element(
            "Tab",
            vec![Attribute::Static {
                name: "label".into(),
                value: "A".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let tab2 = make_element(
            "Tab",
            vec![Attribute::Static {
                name: "label".into(),
                value: "B".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let bar = make_element("TabBar", vec![], vec![Node::Element(tab1), Node::Element(tab2)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        // 应有两次 .child(
        let count = code.matches(".child(").count();
        assert_eq!(count, 2);
        assert!(code.contains(".label(\"A\")"));
        assert!(code.contains(".label(\"B\")"));
    }

    /// 端到端验证：通过 gen_component 入口调用
    #[test]
    fn gen_tab_bar_via_gen_component_dispatch() {
        use crate::compiler::component::gen_component;
        let elem = make_element("TabBar", vec![], vec![]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TabBar::new"));
    }

    /// <tab-bar> kebab-case 标签通过 gen_component 入口调度
    #[test]
    fn gen_tab_bar_kebab_tag() {
        use crate::compiler::component::gen_component;
        let elem = make_element("tab-bar", vec![], vec![]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TabBar::new"));
    }

    /// <tab> 短标签作为 <tab-bar> 子节点
    #[test]
    fn gen_tab_bar_with_tab_short_form() {
        let tab = make_element(
            "tab",
            vec![Attribute::Static {
                name: "label".into(),
                value: "Account".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let bar = make_element("tab-bar", vec![], vec![Node::Element(tab)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".child("));
        assert!(code.contains("rml_ui::Tab::new()"));
        assert!(code.contains(".label(\"Account\")"));
    }

    /// 混合 <Tab> 与 <tab-item> 子节点 —— 验证分派逻辑同时处理两种 item builder
    #[test]
    fn gen_tab_bar_with_tab_item_child() {
        // <TabBar><Tab label="A" /><tab-item title="B"><div>body</div></tab-item></TabBar>
        let tab = make_element(
            "Tab",
            vec![Attribute::Static {
                name: "label".into(),
                value: "A".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let body_div = make_element("div", vec![], vec![Node::Text("body".into())]);
        let item = make_element(
            "tab-item",
            vec![Attribute::Static {
                name: "title".into(),
                value: "B".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(body_div)],
        );
        let bar = make_element("TabBar", vec![], vec![Node::Element(tab), Node::Element(item)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        // Tab 走 .child(rml_ui::Tab::new()...)
        assert!(code.contains(".child(rml_ui::Tab::new()"));
        assert!(code.contains(".label(\"A\")"));
        // TabItem 走 .child(rml_ui::TabItem::new()...)
        assert!(code.contains(".child(rml_ui::TabItem::new()"));
        assert!(code.contains(".title(\"B\")"));
        assert!(code.contains(".body("));
    }

    /// <tab-item each={tab in tabs} title={tab.title}> —— each 循环模式生成 .children(...)
    #[test]
    fn gen_tab_bar_with_tab_item_each() {
        // <TabBar><tab-item each={tab in tabs} title={tab.title} /></TabBar>
        let item = make_element_with_directives(
            "tab-item",
            vec![Attribute::Bind {
                name: "title".into(),
                expr: "tab.title".into(),
                span: Span::empty(),
            }],
            vec![Directive::Each {
                clause: EachClause {
                    item: "tab".into(),
                    index: None,
                    iterable: "tabs".into(),
                },
                span: Span::empty(),
            }],
            vec![],
        );
        let bar = make_element("TabBar", vec![], vec![Node::Element(item)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        // each 模式生成 .children(self.tabs.iter().map(...))
        assert!(code.contains(".children("));
        assert!(code.contains("self.tabs.iter().map(|tab|"));
        assert!(code.contains("let tab = tab.clone();"));
        assert!(code.contains("rml_ui::TabItem::new()"));
        assert!(code.contains(".title(tab.title.clone())"));
        // 不应出现 .child( （each 用 .children）
        assert!(!code.contains("\n            .child("));
    }

    /// <Tab each={tab in tabs} label={tab.title} closable={tab.closable}> —— Tab each 循环模式生成 .children(...)
    #[test]
    fn gen_tab_bar_with_tab_each() {
        // <TabBar><Tab each={tab in tabs} label={tab.title} closable={tab.closable} /></TabBar>
        let tab = make_element_with_directives(
            "Tab",
            vec![
                Attribute::Bind {
                    name: "label".into(),
                    expr: "tab.title".into(),
                    span: Span::empty(),
                },
                Attribute::Bind {
                    name: "closable".into(),
                    expr: "tab.closable".into(),
                    span: Span::empty(),
                },
            ],
            vec![Directive::Each {
                clause: EachClause {
                    item: "tab".into(),
                    index: None,
                    iterable: "tabs".into(),
                },
                span: Span::empty(),
            }],
            vec![],
        );
        let bar = make_element("TabBar", vec![], vec![Node::Element(tab)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        // each 模式生成 .children(self.tabs.iter().map(...))
        assert!(code.contains(".children("));
        assert!(code.contains("self.tabs.iter().map(|tab|"));
        assert!(code.contains("let tab = tab.clone();"));
        assert!(code.contains("rml_ui::Tab::new()"));
        // 循环变量正确解析为 tab.title（非 self.tab.title）
        assert!(code.contains(".label(tab.title.clone())"));
        assert!(code.contains(".closable(tab.closable)"));
        // 不应出现 self.tab（循环变量不应被误解析为 self 字段）
        assert!(!code.contains("self.tab."));
        // 不应出现 .child( （each 用 .children）
        assert!(!code.contains("\n            .child("));
    }
}

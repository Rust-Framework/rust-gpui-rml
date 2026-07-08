//! 原生 TabBar 容器 codegen —— 构造 + 属性 + 子节点 `.child(TabItem::new()...)` 注入。
//!
//! 将 `<TabBar><Tab label="A" /><Tab label="B" /></TabBar>` 转译为
//! `rml_ui::TabBar::new(id).selected_index(0).child(rml_ui::TabItem::new().title("A")).child(rml_ui::TabItem::new().title("B"))`。
//!
//! `<tab>` 子节点 codegen 复用 `tabs::tab::gen_tab_child`（生成 `TabItem::new()...`），
//! 因 TabBar 的 `child()` 接受 `impl Into<TabItem>`。
//!
//! 与 `tabs::gen_tabs` 的关键差异：
//! - 构造器为 `rml_ui::TabBar::new`（非 `rml_ui::Tabs::new`）
//! - 属性走 `tab_bar::setters`（不含 bordered/on_close*/on_promote）
//! - 子节点生成复用 `tabs::tab::gen_tab_child`（相同逻辑）

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 生成 TabBar 构造代码（构造 + 属性 + 子节点 .child(TabItem) 注入）
///
/// 由 `TabBarTranslator` 调用。
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

    // 3. 子节点 → .child(TabItem) 直接构造
    // <tab> 子节点 codegen 复用 tabs::tab::gen_tab_child（生成 TabItem::new()...）
    for child in &elem.children {
        match child {
            Node::Element(child_elem) if tags::is_item_builder_tag(&child_elem.tag) => {
                let (tab_code, is_iter) =
                    super::super::tabs::tab::gen_tab_child(child_elem, ctx, id_counter, loop_vars)?;
                if is_iter {
                    code.push_str(&format!("\n            .children({})", tab_code));
                } else {
                    code.push_str(&format!("\n            .child({})", tab_code));
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
                        "<TabBar> 仅支持 <Tab> 子节点，得到 <{}>",
                        child_elem.tag
                    ),
                    span: Some(elem.span),
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
        assert!(code.contains("rml_ui::TabItem::new()"));
        assert!(code.contains(".title(\"Account\")"));
    }

    #[test]
    fn gen_tab_bar_with_tab_text_child() {
        // <TabBar><Tab>Account</Tab></TabBar> → .child(rml_ui::TabItem::new().title("Account"))
        let tab = make_element("Tab", vec![], vec![Node::Text("Account".into())]);
        let bar = make_element("TabBar", vec![], vec![Node::Element(tab)]);
        let mut id = 0;
        let code = gen_tab_bar(&bar, None, id, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TabItem::new()"));
        assert!(code.contains(".title(\"Account\")"));
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
        assert!(code.contains(".title_icon(rml_ui::IconName::User)"));
        assert!(code.contains(".title(\"Account\")"));
    }

    #[test]
    fn gen_tab_bar_with_on_click() {
        // <TabBar on_click={on_tab_select} /> → .on_click(cx.listener(...))
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
            .contains("仅支持 <Tab> 子节点"));
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
        let count = code.matches(".child(").count();
        assert_eq!(count, 2);
        assert!(code.contains(".title(\"A\")"));
        assert!(code.contains(".title(\"B\")"));
    }
}

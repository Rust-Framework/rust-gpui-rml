//! `<tab-item>` 子节点 codegen — 生成 `TabItem::new().title(...).body(closure)` 表达式。
//!
//! 与 [`super::tab`] 的关键差异：
//! - `<Tab>` 仅有 header（label/icon/children），无 body 概念
//! - `<tab-item>` 同时承载 title (header) 与 body (选中时渲染的内容)，对应 WPF TabControl/TabItem 模式
//!
//! ## body 闭包
//!
//! body 子节点编译为 `Fn(&mut Window, &mut App) -> AnyElement + Send + Sync + 'static` 闭包。
//! - 非 `each` 模式：body 应为静态内容（不引用 `self`），因闭包需 `'static`
//! - `each` 模式：loop 变量 clone 为 owned 后，body 闭包可 capture 之
//!
//! ## `each` 指令
//!
//! `each={tab in tabs}` 生成 `self.tabs.iter().map(|tab| { let tab = tab.clone(); ... })`，
//! loop 变量 clone 为 owned 以满足 body 闭包 `'static` 约束。

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, Element, Node};

/// 为 `<tab-item>` 子节点生成 `rml_ui::TabItem::new()...` 表达式。
///
/// 返回 `(代码, 是否迭代器)`：
/// - 无 `each` 指令：`(构造表达式, false)` → 父用 `.child(...)`
/// - 有 `each` 指令：`(iter().map(...), true)` → 父用 `.children(...)`
pub fn gen_tab_item_child(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<(String, bool), CodegenError> {
    let each_clause = elem.directives.iter().find_map(|d| match d {
        Directive::Each { clause: c, .. } => Some(c.clone()),
        _ => None,
    });

    let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
    if let Some(clause) = &each_clause {
        child_loop_vars.push(clause.item.clone());
        if let Some(idx) = &clause.index {
            child_loop_vars.push(idx.clone());
        }
    }

    let lv: Vec<&str> = child_loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut code = String::from("rml_ui::TabItem::new()");

    let mut title_set_by_attr = false;
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value, "TabItem") {
                    code.push_str(&s);
                    if name == "title" {
                        title_set_by_attr = true;
                    }
                } else if let Some(s) =
                    super::super::component::component_static_setter(name, value, "TabItem")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "TabItem")
                {
                    code.push_str(&s);
                    if name == "title" {
                        title_set_by_attr = true;
                    }
                } else if let Some(s) = super::super::component::component_bind_setter(
                    name, expr, &lv, &computed, "TabItem",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = super::setters::event_setter(name, handler, "TabItem") {
                    code.push_str(&s);
                } else if let Some(s) =
                    super::super::component::component_event_setter(name, handler, "TabItem")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    if !title_set_by_attr {
        for child in &elem.children {
            if let Node::Text(text) = child {
                code.push_str(&format!(".title({:?})", text));
                break;
            }
        }
    }

    let body_children: Vec<&Node> = elem
        .children
        .iter()
        .filter(|c| !matches!(c, Node::Text(_)))
        .collect();

    if !body_children.is_empty() {
        let body_code = if body_children.len() == 1 {
            let (child_code, _) = gen_node(body_children[0], ctx, 0, id_counter, &child_loop_vars)?;
            format!("({}).into_any_element()", child_code)
        } else {
            let mut div_code = String::from("gpui::div()");
            for child in &body_children {
                let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, &child_loop_vars)?;
                if is_iter {
                    div_code.push_str(&format!(".children({})", child_code));
                } else {
                    div_code.push_str(&format!(".child({})", child_code));
                }
            }
            format!("({}).into_any_element()", div_code)
        };

        code.push_str(&format!(
            ".body(move |_window: &mut gpui::Window, _cx: &mut gpui::App| -> gpui::AnyElement {{\n                \
             {}\n            }})",
            body_code
        ));
    }

    if let Some(clause) = each_clause {
        let iter_code = format!(
            "self.{}.iter().map(|{}| {{\n                \
             let {} = {}.clone();\n                \
             {}\n            }})",
            clause.iterable, clause.item, clause.item, clause.item, code
        );
        return Ok((iter_code, true));
    }

    Ok((code, false))
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
    fn gen_tab_item_minimal() {
        // <tab-item title="A" /> → TabItem::new().title("A")
        let elem = make_element(
            "tab-item",
            vec![Attribute::Static {
                name: "title".into(),
                value: "A".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let (code, is_iter) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(!is_iter);
        assert!(code.contains("rml_ui::TabItem::new()"));
        assert!(code.contains(".title(\"A\")"));
    }

    #[test]
    fn gen_tab_item_with_body() {
        // <tab-item title="A"><div>body</div></tab-item>
        let div = make_element("div", vec![], vec![Node::Text("body".into())]);
        let elem = make_element(
            "tab-item",
            vec![Attribute::Static {
                name: "title".into(),
                value: "A".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(div)],
        );
        let mut id = 0;
        let (code, is_iter) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(!is_iter);
        assert!(code.contains(".title(\"A\")"));
        assert!(code.contains(".body("));
        assert!(code.contains("move |_window"));
        assert!(code.contains("into_any_element()"));
    }

    #[test]
    fn gen_tab_item_with_each() {
        // <tab-item each={tab in tabs} title={tab.title} />
        let elem = make_element_with_directives(
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
        let mut id = 0;
        let (code, is_iter) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(is_iter);
        assert!(code.contains("self.tabs.iter().map(|tab|"));
        assert!(code.contains("let tab = tab.clone();"));
        assert!(code.contains(".title(tab.title.clone())"));
    }

    #[test]
    fn gen_tab_item_with_title_icon() {
        // <tab-item title_icon="User" />
        let elem = make_element(
            "tab-item",
            vec![Attribute::Static {
                name: "title_icon".into(),
                value: "User".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let (code, _) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains(".title_icon(rml_ui::IconName::User)"));
    }

    #[test]
    fn gen_tab_item_text_as_title() {
        // <tab-item>Account</tab-item> → .title("Account")
        let elem = make_element("tab-item", vec![], vec![Node::Text("Account".into())]);
        let mut id = 0;
        let (code, _) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains(".title(\"Account\")"));
    }

    #[test]
    fn gen_tab_item_pascal_case_tag() {
        // <TabItem title="A" /> → same as tab-item
        let elem = make_element(
            "TabItem",
            vec![Attribute::Static {
                name: "title".into(),
                value: "A".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let (code, _) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains("rml_ui::TabItem::new()"));
        assert!(code.contains(".title(\"A\")"));
    }

    #[test]
    fn gen_tab_item_with_disabled() {
        // <tab-item title="A" disabled="true" />
        let elem = make_element(
            "tab-item",
            vec![
                Attribute::Static {
                    name: "title".into(),
                    value: "A".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "disabled".into(),
                    value: "true".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let (code, _) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains(".disabled(true)"));
    }

    #[test]
    fn gen_tab_item_each_with_body() {
        // <tab-item each={tab in tabs} title={tab.title}><div>{tab.content}</div></tab-item>
        let div = make_element("div", vec![], vec![Node::Text("body".into())]);
        let elem = make_element_with_directives(
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
            vec![Node::Element(div)],
        );
        let mut id = 0;
        let (code, is_iter) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(is_iter);
        assert!(code.contains("self.tabs.iter().map(|tab|"));
        assert!(code.contains("let tab = tab.clone();"));
        assert!(code.contains(".body("));
    }

    #[test]
    fn gen_tab_item_with_on_click() {
        // <tab-item title="A" on_click={handler} />
        let elem = make_element(
            "tab-item",
            vec![
                Attribute::Static {
                    name: "title".into(),
                    value: "A".into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_click".into(),
                    handler: EventHandler::Ident("on_tab_click".into()),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let (code, _) = gen_tab_item_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains(".title(\"A\")"));
        // on_click 走通用 component_event_setter
        assert!(code.contains(".on_click("));
    }
}

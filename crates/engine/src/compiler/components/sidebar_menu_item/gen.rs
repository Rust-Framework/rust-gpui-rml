//! SidebarMenuItem 构造代码生成
//!
//! ## 构造器
//!
//! `SidebarMenuItem::new(label)` —— `label` 从 `label` 属性提取，无 ElementId。
//!
//! ## 子节点处理
//!
//! 仅支持 `<SidebarMenuItem>` 子节点（子菜单），通过 `.children(vec![...])` 注入。
//! SidebarMenuItem 不实现 `Styled`，不支持 CSS class 样式。
//!
//! ## 属性
//!
//! - `icon` → `.icon(rml_ui::IconName::X)`
//! - `active` → `.active(bool)`
//! - `default_open` → `.default_open(bool)`
//! - `click_to_open` / `click_to_toggle` → `.click_to_open(bool)` / `.click_to_toggle(bool)`
//! - `disabled` → `.disable(bool)`（注意方法名）
//! - `on_click` → 由通用 `component_event_setter` 处理

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 生成 SidebarMenuItem 构造代码
pub fn gen_sidebar_menu_item(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    _parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 提取 label 属性 → 构造器参数
    let label_code = elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Static { name, value, .. } if name == "label" => Some(format!("{:?}", value)),
        Attribute::Bind { name, expr, .. } if name == "label" => {
            let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
            let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
            Some(crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed))
        }
        _ => None,
    }).unwrap_or_else(|| "\"\"".to_string());

    let mut code = format!("rml_ui::SidebarMenuItem::new({})", label_code);

    // 2. 属性 → setter（跳过 label，已由构造器处理）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == "label" => continue,
            Attribute::Bind { name, .. } if name == "label" => continue,
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_static_setter(
                    name, value, "SidebarMenuItem",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) = super::setters::bind_setter(name, expr, &lv, &computed) {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "SidebarMenuItem",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = crate::compiler::setters::component_event_setter(
                    name, handler, "SidebarMenuItem",
                ) {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：仅支持 <SidebarMenuItem> 子菜单，通过 .children(vec![...]) 注入
    let mut submenu_codes: Vec<String> = Vec::new();

    for child in &elem.children {
        match child {
            Node::Element(e) => {
                let canonical = tags::canonical_tag(&e.tag);
                if canonical != "SidebarMenuItem" {
                    return Err(CodegenError {
                        message: format!(
                            "<SidebarMenuItem> 仅支持 <SidebarMenuItem> 子节点（子菜单），得到 <{}>",
                            e.tag
                        ),
                        span: Some(elem.span),
                    });
                }
                let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
                if is_iter {
                    return Err(CodegenError {
                        message: "SidebarMenuItem submenu cannot be each iterators (yet)".into(),
                        span: Some(elem.span),
                    });
                }
                submenu_codes.push(child_code);
            }
            Node::Text(text) => {
                if !text.trim().is_empty() {
                    eprintln!(
                        "[rml warning] <SidebarMenuItem> 不支持文本子节点 {:?}，已忽略",
                        text
                    );
                }
            }
            _ => {}
        }
    }

    if !submenu_codes.is_empty() {
        let joined = submenu_codes.join(", ");
        code.push_str(&format!("\n            .children(vec![{}])", joined));
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Element, EventHandler, Node};
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

    fn make_menu_item(label: &str) -> Element {
        Element {
            tag: "SidebarMenuItem".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: label.into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        }
    }

    #[test]
    fn gen_sidebar_menu_item_minimal() {
        let elem = make_element(
            "SidebarMenuItem",
            vec![Attribute::Static {
                name: "label".into(),
                value: "Home".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::SidebarMenuItem::new(\"Home\")"));
    }

    #[test]
    fn gen_sidebar_menu_item_no_label_defaults_empty() {
        let elem = make_element("SidebarMenuItem", vec![], vec![]);
        let code = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::SidebarMenuItem::new(\"\")"));
    }

    #[test]
    fn gen_sidebar_menu_item_with_icon() {
        let elem = make_element(
            "SidebarMenuItem",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "Settings".into(),
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
        let code = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".icon(rml_ui::IconName::Settings)"));
    }

    #[test]
    fn gen_sidebar_menu_item_active() {
        let elem = make_element(
            "SidebarMenuItem",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "Home".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "active".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".active(true)"));
    }

    #[test]
    fn gen_sidebar_menu_item_disabled() {
        let elem = make_element(
            "SidebarMenuItem",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "Home".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "disabled".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".disable(true)"));
    }

    #[test]
    fn gen_sidebar_menu_item_with_submenu() {
        let elem = make_element(
            "SidebarMenuItem",
            vec![Attribute::Static {
                name: "label".into(),
                value: "Settings".into(),
                span: Span::empty(),
            }],
            vec![
                Node::Element(make_menu_item("General")),
                Node::Element(make_menu_item("Privacy")),
            ],
        );
        let code = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".children(vec!["));
        assert!(code.contains("\"General\""));
        assert!(code.contains("\"Privacy\""));
    }

    #[test]
    fn gen_sidebar_menu_item_rejects_non_menu_item_child() {
        let div = make_element("div", vec![], vec![]);
        let elem = make_element(
            "SidebarMenuItem",
            vec![Attribute::Static {
                name: "label".into(),
                value: "Home".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(div)],
        );
        let result = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("仅支持"));
    }

    #[test]
    fn gen_sidebar_menu_item_label_bind() {
        let elem = make_element(
            "SidebarMenuItem",
            vec![Attribute::Bind {
                name: "label".into(),
                expr: "self.title".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::SidebarMenuItem::new(self.title)"));
    }

    #[test]
    fn gen_sidebar_menu_item_on_click() {
        let elem = make_element(
            "SidebarMenuItem",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "Home".into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_click".into(),
                    handler: EventHandler::Ident("handle_home".into()),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_sidebar_menu_item(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".on_click("));
        assert!(code.contains("cx.listener"));
    }
}

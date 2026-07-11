//! SidebarMenu 构造代码生成
//!
//! ## 构造器
//!
//! `SidebarMenu::new()` —— 无 ElementId、无 cx 参数（RenderOnce 组件）。
//!
//! ## 子节点处理
//!
//! 仅支持 `<SidebarMenuItem>` 子节点，通过 `.child(...)` 注入。
//! 其他元素子节点 → 报错；文本子节点 → 忽略。
//!
//! SidebarMenu 实现 `Styled`，支持 CSS class 样式。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 生成 SidebarMenu 构造代码
pub fn gen_sidebar_menu(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：SidebarMenu::new()
    let mut code = "rml_ui::SidebarMenu::new()".to_string();

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "SidebarMenu", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter（走通用 setter，SidebarMenu 无专用 setter）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "SidebarMenu")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "SidebarMenu",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = crate::compiler::setters::component_event_setter(
                    name, handler, "SidebarMenu",
                ) {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：仅支持 <SidebarMenuItem>，通过 .child() 注入
    for child in &elem.children {
        match child {
            Node::Element(e) => {
                let canonical = tags::canonical_tag(&e.tag);
                if canonical != "SidebarMenuItem" {
                    return Err(CodegenError {
                        message: format!(
                            "<SidebarMenu> 仅支持 <SidebarMenuItem> 子节点，得到 <{}>",
                            e.tag
                        ),
                        span: Some(elem.span),
                    });
                }
                let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
                if is_iter {
                    return Err(CodegenError {
                        message: "SidebarMenu children cannot be each iterators (yet)".into(),
                        span: Some(elem.span),
                    });
                }
                code.push_str(&format!("\n            .child({})", child_code));
            }
            Node::Text(text) => {
                if !text.trim().is_empty() {
                    eprintln!(
                        "[rml warning] <SidebarMenu> 不支持文本子节点 {:?}，已忽略",
                        text
                    );
                }
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
    use crate::parser::ast::{Element, Node};
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    fn make_element(tag: &str, children: Vec<Node>) -> Element {
        Element {
            tag: tag.into(),
            attributes: vec![],
            directives: vec![],
            children,
            slot_name: None,
            ..Default::default()
        }
    }

    fn make_menu_item(label: &str) -> Element {
        Element {
            tag: "SidebarMenuItem".into(),
            attributes: vec![crate::parser::ast::Attribute::Static {
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
    fn gen_sidebar_menu_minimal() {
        let elem = make_element("SidebarMenu", vec![]);
        let code = gen_sidebar_menu(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::SidebarMenu::new()"));
    }

    #[test]
    fn gen_sidebar_menu_with_menu_item() {
        let item = make_menu_item("Home");
        let elem = make_element("SidebarMenu", vec![Node::Element(item)]);
        let code = gen_sidebar_menu(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".child("));
        assert!(code.contains("SidebarMenuItem"));
    }

    #[test]
    fn gen_sidebar_menu_rejects_non_menu_item_child() {
        let div = make_element("div", vec![]);
        let elem = make_element("SidebarMenu", vec![Node::Element(div)]);
        let result = gen_sidebar_menu(&elem, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("仅支持"));
    }

    #[test]
    fn gen_sidebar_menu_ignores_whitespace_text() {
        let elem = make_element("SidebarMenu", vec![Node::Text("  \n  ".into())]);
        let result = gen_sidebar_menu(&elem, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn gen_sidebar_menu_multiple_items() {
        let elem = make_element(
            "SidebarMenu",
            vec![
                Node::Element(make_menu_item("Home")),
                Node::Element(make_menu_item("Settings")),
            ],
        );
        let code = gen_sidebar_menu(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert_eq!(code.matches(".child(").count(), 2);
    }
}

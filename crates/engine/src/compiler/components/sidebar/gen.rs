//! Sidebar 构造代码生成
//!
//! ## 构造器
//!
//! `Sidebar::new(id)` —— 需要 ElementId（Stateless 组件），支持 ref 指令稳定 id。
//!
//! ## 子节点处理
//!
//! - `slot="header"` → `.header(element)`
//! - `slot="footer"` → `.footer(element)`
//! - `<SidebarMenu>` → `.child(rml_ui::SidebarEntry::Menu(...))`
//! - `<SidebarMenuItem>` → `.child(rml_ui::SidebarEntry::Item(...))`
//! - 其他元素子节点 → 报错
//!
//! ## 属性
//!
//! - `side` → `.side(Side::Left/Right)`
//! - `collapsible` → `.collapsible(SidebarCollapsible::Icon/Offcanvas/None)`
//! - `collapsed` → `.collapsed(bool)`

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 生成 Sidebar 构造代码
pub fn gen_sidebar(
    elem: &Element,
    ref_name: Option<&str>,
    id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：Sidebar::new(id)（Stateless，需 ElementId）
    let mut code = if let Some(name) = ref_name {
        format!("rml_ui::Sidebar::new({:?})", format!("rml_ref:{}", name))
    } else {
        format!("rml_ui::Sidebar::new((\"rml_el\", {}usize))", id_val)
    };

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "Sidebar", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Sidebar")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) = super::setters::bind_setter(name, expr, &lv, &computed) {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "Sidebar",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "Sidebar")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点：header slot → .header()，footer slot → .footer()，
    //    SidebarMenu → .child(SidebarEntry::Menu(...))，SidebarMenuItem → .child(SidebarEntry::Item(...))
    let mut header_code: Option<String> = None;
    let mut footer_code: Option<String> = None;
    let mut child_codes: Vec<String> = Vec::new();

    for child in &elem.children {
        match child {
            Node::Element(e) if e.slot_name.as_deref() == Some("header") => {
                let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
                if is_iter {
                    return Err(CodegenError {
                        message: "Sidebar header slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                header_code = Some(child_code);
            }
            Node::Element(e) if e.slot_name.as_deref() == Some("footer") => {
                let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
                if is_iter {
                    return Err(CodegenError {
                        message: "Sidebar footer slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                footer_code = Some(child_code);
            }
            Node::Element(e) => {
                let canonical = tags::canonical_tag(&e.tag);
                let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
                if is_iter {
                    return Err(CodegenError {
                        message: "Sidebar children cannot be each iterators (yet)".into(),
                        span: Some(elem.span),
                    });
                }
                match canonical.as_str() {
                    "SidebarMenu" => {
                        child_codes.push(format!(
                            ".child(rml_ui::SidebarEntry::Menu({}))",
                            child_code
                        ));
                    }
                    "SidebarMenuItem" => {
                        child_codes.push(format!(
                            ".child(rml_ui::SidebarEntry::Item({}))",
                            child_code
                        ));
                    }
                    _ => {
                        return Err(CodegenError {
                            message: format!(
                                "<Sidebar> 仅支持 <SidebarMenu> 或 <SidebarMenuItem> 子节点，得到 <{}>",
                                e.tag
                            ),
                            span: Some(elem.span),
                        });
                    }
                }
            }
            Node::Text(text) => {
                if !text.trim().is_empty() {
                    eprintln!(
                        "[rml warning] <Sidebar> 不支持文本子节点 {:?}，已忽略",
                        text
                    );
                }
            }
            _ => {}
        }
    }

    // 先注入 header，再注入 footer，最后注入子节点
    if let Some(hc) = header_code {
        code.push_str(&format!("\n            .header({})", hc));
    }
    if let Some(fc) = footer_code {
        code.push_str(&format!("\n            .footer({})", fc));
    }
    for child_code in child_codes {
        code.push_str(&format!("\n            {}", child_code));
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Directive, Element, Node};
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

    fn make_slotted(tag: &str, slot: &str, children: Vec<Node>) -> Element {
        Element {
            tag: tag.into(),
            attributes: vec![],
            directives: vec![],
            children,
            slot_name: Some(slot.into()),
            ..Default::default()
        }
    }

    #[test]
    fn gen_sidebar_minimal() {
        let elem = make_element("Sidebar", vec![], vec![]);
        let code = gen_sidebar(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Sidebar::new"));
        assert!(code.contains("\"rml_el\""));
    }

    #[test]
    fn gen_sidebar_with_ref_uses_stable_id() {
        let elem = make_element_with_directives(
            "Sidebar",
            vec![],
            vec![Directive::Ref {
                name: "my_sidebar".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sidebar(&elem, Some("my_sidebar"), 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Sidebar::new(\"rml_ref:my_sidebar\")"));
        assert!(!code.contains("rml_el"));
    }

    #[test]
    fn gen_sidebar_side_left() {
        let elem = make_element(
            "Sidebar",
            vec![Attribute::Static {
                name: "side".into(),
                value: "left".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sidebar(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".side(rml_ui::Side::Left)"));
    }

    #[test]
    fn gen_sidebar_collapsible_offcanvas() {
        let elem = make_element(
            "Sidebar",
            vec![Attribute::Static {
                name: "collapsible".into(),
                value: "offcanvas".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sidebar(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".collapsible(rml_ui::SidebarCollapsible::Offcanvas)"));
    }

    #[test]
    fn gen_sidebar_collapsed_bind() {
        let elem = make_element(
            "Sidebar",
            vec![Attribute::Bind {
                name: "collapsed".into(),
                expr: "is_collapsed".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sidebar(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".collapsed(is_collapsed)"));
    }

    #[test]
    fn gen_sidebar_with_header_slot() {
        let header = make_slotted("div", "header", vec![Node::Text("Header".into())]);
        let elem = make_element("Sidebar", vec![], vec![Node::Element(header)]);
        let code = gen_sidebar(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".header("));
    }

    #[test]
    fn gen_sidebar_with_footer_slot() {
        let footer = make_slotted("div", "footer", vec![Node::Text("Footer".into())]);
        let elem = make_element("Sidebar", vec![], vec![Node::Element(footer)]);
        let code = gen_sidebar(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".footer("));
    }

    #[test]
    fn gen_sidebar_rejects_unsupported_child() {
        let div = make_element("div", vec![], vec![]);
        let elem = make_element("Sidebar", vec![], vec![Node::Element(div)]);
        let result = gen_sidebar(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("仅支持"));
    }

    #[test]
    fn gen_sidebar_ignores_whitespace_text() {
        let elem = make_element("Sidebar", vec![], vec![Node::Text("  \n  ".into())]);
        let result = gen_sidebar(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_ok());
    }
}

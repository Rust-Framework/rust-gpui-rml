//! Tag 组件 codegen
//!
//! Tag 的 variant 属性（primary/secondary/danger/success/warning/info）是关联函数
//! 而非方法，需在构造器选择阶段决定使用 `Tag::new()` 还是 `Tag::primary()` 等。
//!
//! 其他属性（size/disabled/compact 等）走通用 setter。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};
use crate::tags;

/// 生成 Tag 构造代码
pub fn gen_tag(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 扫描 variant 属性，决定构造器
    let mut ctor = "rml_ui::Tag::new()".to_string();
    for attr in &elem.attributes {
        if let Attribute::Static { name, value, .. } = attr {
            if is_variant_attr(name) && (value.is_empty() || value.eq_ignore_ascii_case("true")) {
                ctor = format!("rml_ui::Tag::{}()", name);
                break;
            }
        }
    }

    let mut code = ctor;

    // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
    append_css_class_styles(&mut code, elem, "Tag", ctx.stylesheet.as_ref(), parents);

    // 2. 处理其他属性（跳过 variant 属性，已用于构造器）
    let resolved = tags::normalize_component_tag(&elem.tag);
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                // variant 属性已用于构造器，跳过
                if is_variant_attr(name) {
                    continue;
                }
                // Tag 专用：outline="" → .outline()（描边样式，透明背景）
                if name == "outline" && (value.is_empty() || value.eq_ignore_ascii_case("true")) {
                    code.push_str(".outline()");
                    continue;
                }
                if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, &resolved)
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, &resolved,
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, &resolved)
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点处理
    //
    // Tag 实现 ParentElement，用 .child(...) 接收子节点（文本/元素）。
    // each 指令生成的迭代器用 .children(...) 包裹。
    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!(".children({})", child_code));
        } else {
            code.push_str(&format!(".child({})", child_code));
        }
    }

    Ok(code)
}

fn is_variant_attr(name: &str) -> bool {
    matches!(
        name,
        "primary" | "secondary" | "danger" | "success" | "warning" | "info"
    )
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
    fn gen_tag_default() {
        let elem = make_element("Tag", vec![], vec![]);
        let mut id = 0;
        let code = gen_tag(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Tag::new()"));
    }

    #[test]
    fn gen_tag_primary_variant() {
        let elem = make_element(
            "Tag",
            vec![Attribute::Static {
                name: "primary".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_tag(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Tag::primary()"));
        // 确保没有重复的 .primary() 调用
        assert!(!code.contains(".primary()"));
    }

    #[test]
    fn gen_tag_danger_variant() {
        let elem = make_element(
            "Tag",
            vec![Attribute::Static {
                name: "danger".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_tag(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Tag::danger()"));
    }

    #[test]
    fn gen_tag_with_size() {
        let elem = make_element(
            "Tag",
            vec![
                Attribute::Static {
                    name: "primary".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "size".into(),
                    value: "small".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_tag(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Tag::primary()"));
        assert!(code.contains(".with_size(rml_ui::Size::Small)"));
    }

    #[test]
    fn gen_tag_outline() {
        let elem = make_element(
            "Tag",
            vec![
                Attribute::Static {
                    name: "primary".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "outline".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_tag(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Tag::primary()"));
        assert!(code.contains(".outline()"));
    }
}

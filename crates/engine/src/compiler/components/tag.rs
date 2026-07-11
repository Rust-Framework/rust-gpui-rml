//! Tag 组件 codegen
//!
//! Tag 构造器统一为 `Tag::new()`，variant 通过独立布尔属性
//! `primary` / `secondary` / `danger` / `success` / `warning` / `info`
//! 映射到 `.with_variant(TagVariant::*)`。
//!
//! 其他属性（size/outline 等）走通用 setter 或 Tag 专属 setter。

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
    // 1. 构造器统一为 Tag::new()，variant 由独立布尔属性 + .with_variant() 设置
    let mut code = "rml_ui::Tag::new()".to_string();

    // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
    append_css_class_styles(&mut code, elem, "Tag", ctx.stylesheet.as_ref(), parents);

    // 2. 处理属性
    let resolved = tags::normalize_component_tag(&elem.tag);
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                // variant 布尔属性: primary/secondary/danger/success/warning/info → .with_variant(TagVariant::*)
                if let Some(variant_name) = tag_variant_from_attr(name) {
                    if value.is_empty() || value.eq_ignore_ascii_case("true") {
                        code.push_str(&format!(".with_variant(rml_ui::TagVariant::{})", variant_name));
                    }
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

/// Tag variant 布尔属性名 → TagVariant 枚举变体名
///
/// `primary` → `Primary`，`secondary` → `Secondary`，`danger` → `Danger`，
/// `success` → `Success`，`warning` → `Warning`，`info` → `Info`
fn tag_variant_from_attr(name: &str) -> Option<&'static str> {
    match name {
        "primary" => Some("Primary"),
        "secondary" => Some("Secondary"),
        "danger" => Some("Danger"),
        "success" => Some("Success"),
        "warning" => Some("Warning"),
        "info" => Some("Info"),
        _ => None,
    }
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
        // <Tag primary /> → Tag::new().with_variant(TagVariant::Primary)
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
        assert!(code.contains("rml_ui::Tag::new()"));
        assert!(code.contains(".with_variant(rml_ui::TagVariant::Primary)"));
    }

    #[test]
    fn gen_tag_danger_variant() {
        // <Tag danger /> → Tag::new().with_variant(TagVariant::Danger)
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
        assert!(code.contains("rml_ui::Tag::new()"));
        assert!(code.contains(".with_variant(rml_ui::TagVariant::Danger)"));
    }

    #[test]
    fn gen_tag_with_size() {
        // <Tag primary size="small" />
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
        assert!(code.contains(".with_variant(rml_ui::TagVariant::Primary)"));
        assert!(code.contains(".with_size(rml_ui::Size::Small)"));
    }

    #[test]
    fn gen_tag_outline() {
        // <Tag primary outline />
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
        assert!(code.contains(".with_variant(rml_ui::TagVariant::Primary)"));
        assert!(code.contains(".outline()"));
    }
}

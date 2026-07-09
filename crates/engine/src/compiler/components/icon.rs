//! Icon 组件代码生成
//!
//! Icon 构造器：`Icon::new(impl Into<Icon>)`，接受 `IconName` 或 path 字符串。
//! Icon 是 RenderOnce，无 ElementId。
//!
//! ## 属性映射
//!
//! - `name="Settings"` → `Icon::new(rml_ui::IconName::Settings)`（构造器参数）
//! - `path="icons/foo.svg"` → `Icon::empty().path("icons/foo.svg")`（构造器参数）
//! - `name` 与 `path` 互斥，`name` 优先；两者都未提供时使用 `Icon::empty()`
//! - `size="small"` 等通用属性走 `component_static_setter` 链

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element};
use crate::tags;

/// 生成 Icon 构造代码
pub fn gen_icon(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let resolved = "Icon";
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. 构造器：name 优先，path 次之，都未提供则 empty
    let mut code = String::new();
    let mut name_set = false;
    let mut path_set = false;

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == "name" && !path_set => {
                // name="Settings" → Icon::new(rml_ui::IconName::Settings)
                code.push_str(&format!("rml_ui::Icon::new(rml_ui::IconName::{})", value));
                name_set = true;
            }
            Attribute::Bind { name, expr, .. } if name == "name" && !path_set => {
                // name={icon_name_field} → Icon::new(icon_name_field.clone())
                // 注：IconName 是 Copy，但字段引用需 clone 避免 move
                let rust_expr =
                    super::component::component_bind_rust_expr(expr, &lv, &computed);
                code.push_str(&format!("rml_ui::Icon::new({})", rust_expr));
                name_set = true;
            }
            Attribute::Static { name, value, .. } if name == "path" && !name_set => {
                // path="icons/foo.svg" → Icon::empty().path("icons/foo.svg")
                code.push_str(&format!("rml_ui::Icon::empty().path({:?})", value));
                path_set = true;
            }
            Attribute::Bind { name, expr, .. } if name == "path" && !name_set => {
                let rust_expr =
                    super::component::component_bind_rust_expr(expr, &lv, &computed);
                code.push_str(&format!("rml_ui::Icon::empty().path({})", rust_expr));
                path_set = true;
            }
            _ => {}
        }
    }

    // 回退：无 name/path 时使用 empty
    if !name_set && !path_set {
        code.push_str("rml_ui::Icon::empty()");
    }

    // 2. 其他属性 → builder 方法（size/color 等走通用 setter）
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if name == "name" || name == "path" {
                    continue;
                }
                if let Some(s) =
                    super::component::component_static_setter(name, value, resolved)
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if name == "name" || name == "path" {
                    continue;
                }
                if let Some(s) = super::component::component_bind_setter(
                    name, expr, &lv, &computed, resolved,
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    super::component::component_event_setter(name, handler, resolved)
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点处理：Icon 实现 ParentElement，可接受元素子节点作为叠加内容
    let _ = id_counter;
    let _ = tags::canonical_tag(&elem.tag);
    for child in &elem.children {
        if let crate::parser::ast::Node::Text(t) = child {
            code.push_str(&format!(".child(gpui::Styled::text_size(gpui::div(), 0.).child({:?}))", t));
            break;
        }
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Element, Node};
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
    fn gen_icon_name_static() {
        // <Icon name="Settings" />
        let elem = make_element(
            "Icon",
            vec![Attribute::Static {
                name: "name".into(),
                value: "Settings".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_icon(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Icon::new(rml_ui::IconName::Settings)"));
    }

    #[test]
    fn gen_icon_path_static() {
        // <Icon path="icons/foo.svg" />
        let elem = make_element(
            "Icon",
            vec![Attribute::Static {
                name: "path".into(),
                value: "icons/foo.svg".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_icon(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Icon::empty().path(\"icons/foo.svg\")"));
    }

    #[test]
    fn gen_icon_name_priority_over_path() {
        // <Icon name="Bell" path="ignored.svg" /> → name 优先
        let elem = make_element(
            "Icon",
            vec![
                Attribute::Static {
                    name: "name".into(),
                    value: "Bell".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "path".into(),
                    value: "ignored.svg".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_icon(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Icon::new(rml_ui::IconName::Bell)"));
        // path 不应被生成（被 name 优先跳过）
        assert!(!code.contains("ignored.svg"));
    }

    #[test]
    fn gen_icon_no_name_no_path_uses_empty() {
        // <Icon size="small" /> → 无 name/path，回退到 empty
        let elem = make_element(
            "Icon",
            vec![Attribute::Static {
                name: "size".into(),
                value: "small".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_icon(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Icon::empty()"));
        // size 走通用 setter 链
        assert!(code.contains(".with_size(rml_ui::Size::Small)"));
    }

    #[test]
    fn gen_icon_name_with_size() {
        // <Icon name="Settings" size="large" />
        let elem = make_element(
            "Icon",
            vec![
                Attribute::Static {
                    name: "name".into(),
                    value: "Settings".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "size".into(),
                    value: "large".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_icon(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Icon::new(rml_ui::IconName::Settings)"));
        assert!(code.contains(".with_size(rml_ui::Size::Large)"));
    }

    #[test]
    fn gen_icon_name_bind() {
        // <Icon name={icon_name} /> → Icon::new(self.icon_name.clone())
        let elem = make_element(
            "Icon",
            vec![Attribute::Bind {
                name: "name".into(),
                expr: "icon_name".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_icon(&elem, &ctx(), &mut 0, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Icon::new(self.icon_name)"));
    }
}

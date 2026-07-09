//! Separator 组件代码生成
//!
//! Separator 无 `new()` 构造器，使用关联函数：
//! - `Separator::horizontal()` (默认)
//! - `Separator::vertical()`
//! - `Separator::horizontal_dashed()`
//! - `Separator::vertical_dashed()`
//!
//! 通过 `vertical` / `dashed` 属性组合选择构造器。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

/// 生成 Separator 构造代码
///
/// 属性组合：
/// - `<Separator />` → `Separator::horizontal()`
/// - `<Separator vertical="" />` → `Separator::vertical()`
/// - `<Separator dashed="" />` → `Separator::horizontal_dashed()`
/// - `<Separator vertical="" dashed="" />` → `Separator::vertical_dashed()`
pub fn gen_separator(
    elem: &Element,
    ctx: &CodegenCtx,
    _id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let resolved = "Separator";
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut is_vertical = false;
    let mut is_dashed = false;

    for attr in &elem.attributes {
        if let Attribute::Static { name, value, .. } = attr {
            if name == "vertical" && (value.is_empty() || value.eq_ignore_ascii_case("true")) {
                is_vertical = true;
            }
            if name == "dashed" && (value.is_empty() || value.eq_ignore_ascii_case("true")) {
                is_dashed = true;
            }
        }
    }

    let ctor = match (is_vertical, is_dashed) {
        (false, false) => "Separator::horizontal()",
        (true, false) => "Separator::vertical()",
        (false, true) => "Separator::horizontal_dashed()",
        (true, true) => "Separator::vertical_dashed()",
    };

    let mut code = format!("rml_ui::{}", ctor);

    // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
    append_css_class_styles(&mut code, elem, "Separator", ctx.stylesheet.as_ref(), parents);

    // 其他属性 → builder 方法（跳过 vertical/dashed，已用于构造器选择）
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if name == "vertical" || name == "dashed" {
                    continue;
                }
                if let Some(setter) =
                    crate::compiler::setters::component_static_setter(name, value, resolved)
                {
                    code.push_str(&setter);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(setter) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, resolved,
                ) {
                    code.push_str(&setter);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(setter) =
                    crate::compiler::setters::component_event_setter(name, handler, resolved)
                {
                    code.push_str(&setter);
                }
            }
        }
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            ..Default::default()
        }
    }

    fn make_element(attrs: Vec<Attribute>) -> Element {
        Element {
            tag: "Separator".into(),
            attributes: attrs,
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        }
    }

    #[test]
    fn gen_separator_default_horizontal() {
        let elem = make_element(vec![]);
        let mut id = 0;
        let code = gen_separator(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("Separator::horizontal()"));
    }

    #[test]
    fn gen_separator_vertical() {
        let elem = make_element(vec![Attribute::Static {
            name: "vertical".into(),
            value: "".into(),
            span: Span::empty(),
        }]);
        let mut id = 0;
        let code = gen_separator(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("Separator::vertical()"));
    }

    #[test]
    fn gen_separator_dashed() {
        let elem = make_element(vec![Attribute::Static {
            name: "dashed".into(),
            value: "".into(),
            span: Span::empty(),
        }]);
        let mut id = 0;
        let code = gen_separator(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("Separator::horizontal_dashed()"));
    }

    #[test]
    fn gen_separator_vertical_dashed() {
        let elem = make_element(vec![
            Attribute::Static {
                name: "vertical".into(),
                value: "".into(),
                span: Span::empty(),
            },
            Attribute::Static {
                name: "dashed".into(),
                value: "".into(),
                span: Span::empty(),
            },
        ]);
        let mut id = 0;
        let code = gen_separator(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("Separator::vertical_dashed()"));
    }
}

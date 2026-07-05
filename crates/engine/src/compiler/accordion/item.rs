//! AccordionItem 闭包式 builder 代码生成。
//!
//! 生成 `|__rml_item: rml_ui::AccordionItem| __rml_item.<setters>.child(...)` 闭包，
//! 由 `accordion::gen_accordion` 为每个 `<AccordionItem>` 子节点调用。

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element};

/// 为 `<AccordionItem>` 子节点生成闭包式 builder 代码
///
/// 生成形如：
/// ```text
/// |__rml_item: rml_ui::AccordionItem| __rml_item.title("Section 1").open(true).child("Content")
/// ```
pub fn gen_item_builder(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut code = String::from("|__rml_item: rml_ui::AccordionItem| __rml_item");

    // 静态/绑定属性 → 先调 accordion 专用 setter，未命中回退到公共 setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) =
                    super::setters::static_setter(name, value, "AccordionItem")
                {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_static_setter(
                    name, value, "AccordionItem",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) = super::setters::bind_setter(
                    name, expr, &lv, &computed, "AccordionItem",
                ) {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_bind_setter(
                    name, expr, &lv, &computed, "AccordionItem",
                ) {
                    code.push_str(&s);
                }
            }
            // AccordionItem 当前无事件属性，跳过
            _ => {}
        }
    }

    // 子节点 → .child(...) / .children(...)
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

//! 菜单子节点拆分与触发器子节点生成。
//!
//! - `partition_menu_children`：将子节点分为触发器元素与菜单项元素
//! - `gen_trigger_children`：递归生成触发器子节点代码

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Element, Node};

use super::item::is_menu_item_tag;

/// 将子节点分为触发器元素与菜单项元素
pub(crate) fn partition_menu_children(
    children: &[Node],
) -> (Vec<&Element>, Vec<&Element>) {
    let mut triggers = Vec::new();
    let mut items = Vec::new();
    for child in children {
        if let Node::Element(elem) = child {
            if is_menu_item_tag(&elem.tag) {
                items.push(elem);
            } else {
                triggers.push(elem);
            }
        }
    }
    (triggers, items)
}

/// 递归生成触发器子节点代码
pub(crate) fn gen_trigger_children(
    triggers: &[&Element],
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    if triggers.is_empty() {
        return Ok("gpui::div()".to_string());
    }
    if triggers.len() == 1 {
        let (code, _) = gen_node(&Node::Element(triggers[0].clone()), ctx, depth, id_counter, loop_vars)?;
        return Ok(code);
    }
    let mut parts = Vec::new();
    for t in triggers {
        let (code, _) = gen_node(&Node::Element((*t).clone()), ctx, depth, id_counter, loop_vars)?;
        parts.push(code);
    }
    Ok(format!(
        "gpui::div().flex().flex_col().children(vec![{}])",
        parts.join(", ")
    ))
}

//! RML 菜单 codegen —— 将声明式 `MenuItem` / `MenuSeparator` 转译为 gpui-component PopupMenu API。
//!
//! 子标签仅两种：`MenuItem`、`MenuSeparator`（菜单上下文中 `Separator` 为别名）。

mod app_menu_bar;
mod context;
mod dropdown;
mod hoist;
mod item;
mod menu_bar;
mod popup;

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Element, Node};

use crate::tags;

pub use item::is_menu_item_tag;

/// 菜单容器标签（PascalCase 或 kebab-case）
pub fn is_menu_container(tag: &str) -> bool {
    matches!(
        tags::normalize_component_tag(tag).as_str(),
        "ContextMenu" | "DropdownMenu" | "MenuBar" | "AppMenuBar" | "menu"
    )
}

/// 菜单相关标签（容器 + 子项）
pub fn is_menu_tag(tag: &str) -> bool {
    is_menu_container(tag) || is_menu_item_tag(tag)
}

/// 生成菜单相关元素代码
pub fn gen_menu_element(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let canonical = tags::normalize_component_tag(&elem.tag);
    match canonical.as_str() {
        "ContextMenu" => context::gen_context_menu(elem, ctx, depth, id_counter, loop_vars),
        "DropdownMenu" => dropdown::gen_dropdown_menu(elem, ctx, depth, id_counter, loop_vars),
        "MenuBar" | "menu" => menu_bar::gen_menu_bar(elem, ctx, depth, id_counter, loop_vars),
        "AppMenuBar" => app_menu_bar::gen_app_menu_bar(elem, ctx),
        tag if is_menu_item_tag(tag) => Err(CodegenError {
            message: format!(
                "<{tag}> must be a child of context-menu, dropdown-menu, or menu-bar"
            ),
        }),
        _ => Err(CodegenError {
            message: format!("unknown menu tag: <{}>", elem.tag),
        }),
    }
}

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

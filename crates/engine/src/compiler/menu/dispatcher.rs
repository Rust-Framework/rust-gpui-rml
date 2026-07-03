//! 菜单元素 codegen 分发器。
//!
//! - `is_menu_container` / `is_menu_tag`：标签识别
//! - `gen_menu_element`：根据规范化标签分发到具体子模块（context / dropdown / menu_bar / app_menu_bar）

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::Element;
use crate::tags;

use super::item::is_menu_item_tag;

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
        "ContextMenu" => super::context::gen_context_menu(elem, ctx, depth, id_counter, loop_vars),
        "DropdownMenu" => super::dropdown::gen_dropdown_menu(elem, ctx, depth, id_counter, loop_vars),
        "MenuBar" | "menu" => super::menu_bar::gen_menu_bar(elem, ctx, depth, id_counter, loop_vars),
        "AppMenuBar" => super::app_menu_bar::gen_app_menu_bar(elem, ctx),
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

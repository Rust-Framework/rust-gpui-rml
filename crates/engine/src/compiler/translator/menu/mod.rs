//! 菜单容器 translator
//!
//! 每个菜单容器类型独占一个 translator 文件：
//! - `context_menu`：`<ContextMenu>` / `<context-menu>`
//! - `dropdown_menu`：`<DropdownMenu>` / `<dropdown-menu>`
//! - `menu_bar`：`<MenuBar>` / `<menu-bar>` / `<menu>`
//! - `app_menu_bar`：`<AppMenuBar>` / `<app-menu-bar>`

pub mod app_menu_bar;
pub mod context_menu;
pub mod dropdown_menu;
pub mod menu_bar;

pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    context_menu::register(registry);
    dropdown_menu::register(registry);
    menu_bar::register(registry);
    app_menu_bar::register(registry);
}

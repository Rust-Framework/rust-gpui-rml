//! 菜单栏渲染器 —— 将 `Vec<MenuItem>` 数据渲染为水平菜单栏（支持下拉/子菜单）
//!
//! 顶层菜单项水平排列。带子菜单的项点击后弹出 `PopupMenu` 下拉，
//! 支持递归子菜单、分隔符、勾选状态、禁用状态。
//!
//! 基于 `gpui_component::menu::DropdownMenu` trait（`Button::dropdown_menu`），
//! 由 `DropdownMenuPopover` 内部管理打开/关闭状态与 dismiss 订阅，
//! 因此 `ModernWindowShell`（RenderOnce 无状态）可直接使用，无需 ViewModel 持有菜单状态。

use gpui::{AnyElement, IntoElement, ParentElement, Styled, Window, prelude::FluentBuilder as _};
use gpui_component::{
    Disableable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
};

use super::types::MenuItem;

/// 渲染水平菜单栏
pub fn render_menu_bar(items: &[MenuItem]) -> impl IntoElement {
    gpui::div()
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .children(items.iter().enumerate().map(|(i, item)| {
            render_menu_item(i, item)
        }))
}

fn render_menu_item(idx: usize, item: &MenuItem) -> AnyElement {
    let label = item.label.clone();

    if item.children.is_empty() {
        // 叶子节点：ghost 按钮直接触发 on_click
        let on_click = item.on_click.clone();
        Button::new(("menu-item", idx))
            .small()
            .compact()
            .ghost()
            .label(label)
            .disabled(item.disabled)
            .when_some(on_click, |this, handler| {
                this.on_click(move |_, window, cx| handler(window, cx))
            })
            .into_any_element()
    } else {
        // 含子菜单：dropdown_menu 弹出 PopupMenu
        let children = item.children.clone();
        Button::new(("menu-item", idx))
            .small()
            .compact()
            .ghost()
            .label(label)
            .disabled(item.disabled)
            .dropdown_menu(move |menu, window, cx| {
                build_popup_menu(menu, &children, window, cx)
            })
            .into_any_element()
    }
}

/// 递归将 `&[MenuItem]` 转换为 `PopupMenu` builder 链
///
/// `cx` 是 `&mut Context<PopupMenu>`（`dropdown_menu`/`submenu` builder 提供）
fn build_popup_menu(
    mut menu: PopupMenu,
    items: &[MenuItem],
    window: &mut Window,
    cx: &mut gpui::Context<PopupMenu>,
) -> PopupMenu {
    for item in items {
        if item.separator {
            menu = menu.separator();
            continue;
        }
        if item.children.is_empty() {
            let mut pitem = PopupMenuItem::new(item.label.clone())
                .disabled(item.disabled)
                .checked(item.checked);
            if let Some(h) = item.on_click.clone() {
                // MenuItem.on_click: Rc<dyn Fn(&mut Window, &mut App)>
                // PopupMenuItem::on_click: Fn(&ClickEvent, &mut Window, &mut App)
                pitem = pitem.on_click(move |_, window, cx| h(window, cx));
            }
            menu = menu.item(pitem);
        } else {
            let sub_children = item.children.clone();
            menu = menu.submenu(item.label.clone(), window, cx, move |m, window, cx| {
                build_popup_menu(m, &sub_children, window, cx)
            });
        }
    }
    menu
}

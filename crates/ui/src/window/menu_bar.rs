//! 菜单栏渲染器 —— 将 `Vec<MenuItem>` 数据渲染为水平菜单栏
//!
//! Phase 1 简化实现：顶层菜单项水平排列，点击触发 `on_click` 闭包。
//! 子菜单与下拉列表留待 Phase 4 集成 `PopupMenu`。

use gpui::{
    App, ClickEvent, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, Window, div, prelude::FluentBuilder as _,
};

use super::types::MenuItem;

/// 渲染水平菜单栏
pub fn render_menu_bar(items: &[MenuItem]) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .children(items.iter().enumerate().map(|(i, item)| {
            render_menu_item(i, item)
        }))
}

fn render_menu_item(idx: usize, item: &MenuItem) -> impl IntoElement {
    let click_handler = item.on_click.clone();
    let label = item.label.clone();
    div()
        .id(("menu-item", idx))
        .px_2()
        .py_0p5()
        .text_sm()
        .cursor_pointer()
        .rounded_sm()
        .hover(|s| s.opacity(0.7))
        .when_some(click_handler, |this, handler| {
            this.on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                handler(window, cx);
            })
        })
        .child(label)
}

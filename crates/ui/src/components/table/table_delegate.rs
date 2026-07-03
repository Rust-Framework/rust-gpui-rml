//! TableDelegate trait —— 表格模板委托（WPF DataTemplate 等价）
//!
//! 用于自定义列头和单元格渲染。当声明式插槽模板（`<template slot="cell">`）
//! 无法满足需求（如需事件处理 `onclick={self.method}`）时，用户可在 .rml.rs
//! 中实现 TableDelegate，通过 `delegate={table_delegate}` 绑定注入。
//!
//! 渲染优先级：`cell_templates`（声明式插槽）> `delegate` > `DefaultTableDelegate`
//!
//! # 示例
//!
//! ```ignore
//! struct UserTableDelegate;
//! impl TableDelegate for UserTableDelegate {
//!     fn render_cell(&self, row: usize, col: usize, column: &TableColumn,
//!                    row_data: &TableRow, cx: &mut App) -> AnyElement {
//!         if column.key == "actions" {
//!             Button::new(("edit", row)).label("Edit").into_any_element()
//!         } else {
//!             let text = row_data.get(&column.key);
//!             div().child(text).into_any_element()
//!         }
//!     }
//! }
//! ```

use gpui::{AnyElement, App, IntoElement, ParentElement, div};

use super::table_column::TableColumn;
use super::table_row::TableRow;

/// 表格模板委托 —— 支持自定义列头和单元格渲染
pub trait TableDelegate: 'static {
    /// 渲染列头。默认实现返回 `column.title` 文本。
    fn render_header(
        &self,
        _col: usize,
        column: &TableColumn,
        _cx: &mut App,
    ) -> AnyElement {
        div().child(column.title.clone()).into_any_element()
    }

    /// 渲染单元格。默认实现返回 `row_data.cells[column.key]` 文本。
    fn render_cell(
        &self,
        _row: usize,
        _col: usize,
        column: &TableColumn,
        row_data: &TableRow,
        _cx: &mut App,
    ) -> AnyElement {
        let text = row_data.get(&column.key);
        div().child(text).into_any_element()
    }
}

/// 默认委托（纯文本渲染）
///
/// `Table` 在未提供 delegate 且无声明式插槽模板时使用此类型。
pub struct DefaultTableDelegate;

impl TableDelegate for DefaultTableDelegate {}

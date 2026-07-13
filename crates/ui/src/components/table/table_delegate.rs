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

use std::sync::Arc;

use gpui::{div, AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window};

use super::table_column::TableColumn;
use super::table_row::TableRow;

/// 表格模板委托 —— 支持自定义列头和单元格渲染
pub trait TableDelegate: 'static + Send + Sync {
    /// 渲染列头。默认实现返回 `column.title` 文本。
    fn render_header(&self, _col: usize, column: &TableColumn, _cx: &mut App) -> AnyElement {
        div()
            .whitespace_nowrap()
            .overflow_hidden()
            .child(column.title.clone())
            .into_any_element()
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
        div()
            .whitespace_nowrap()
            .overflow_hidden()
            .child(text)
            .into_any_element()
    }

    /// 是否允许编辑指定单元格。默认实现返回 `column.editable`。
    fn can_edit(&self, _row: usize, _col: usize, column: &TableColumn) -> bool {
        column.editable
    }

    /// 指定单元格是否处于编辑模式。默认实现返回 `false`。
    /// 用户应通过 `Mutex<Option<(usize, usize)>>` 跟踪编辑状态并覆写此方法。
    fn is_editing(&self, _row: usize, _col: usize) -> bool {
        false
    }

    /// 进入编辑模式。默认实现为空操作。
    /// 用户应覆写此方法记录正在编辑的 (row, col)，并在内部触发重新渲染。
    fn start_edit(&self, _row: usize, _col: usize) {}

    /// 退出编辑模式。默认实现为空操作。
    fn stop_edit(&self) {}

    /// 设置重新渲染通知回调。Table 在每次 render 时调用此方法，
    /// 将通知回调注入 delegate，使 delegate 在编辑状态变更时能触发重新渲染。
    /// 默认实现为空操作，用户应覆写此方法存储回调。
    fn set_notify(&self, _notify: Arc<dyn Fn(&mut App) + Send + Sync>) {}

    /// 渲染编辑器。当单元格处于编辑模式时调用，返回编辑器元素（如 Input）。
    /// 默认实现返回空 div，用户应覆写此方法提供自定义编辑器。
    /// 编辑器应自行处理 Enter 提交 / Escape 取消 / Blur 提交等事件，
    /// 并在提交时调用 `self.on_cell_commit()` + `self.stop_edit()` + 通知回调。
    fn render_editor(
        &self,
        _row: usize,
        _col: usize,
        _column: &TableColumn,
        _row_data: &TableRow,
        _window: &mut Window,
        _cx: &mut App,
    ) -> AnyElement {
        div().into_any_element()
    }

    /// 单元格编辑提交回调。编辑完成时调用，`new_value` 为编辑后的新值。
    /// 默认实现为空操作，用户应覆写此方法处理数据更新。
    fn on_cell_commit(&self, _row: usize, _col: usize, _new_value: SharedString, _cx: &mut App) {}
}

/// 默认委托（纯文本渲染）
///
/// `Table` 在未提供 delegate 且无声明式插槽模板时使用此类型。
pub struct DefaultTableDelegate;

impl TableDelegate for DefaultTableDelegate {}

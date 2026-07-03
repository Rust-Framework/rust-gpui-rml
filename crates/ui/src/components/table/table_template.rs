//! Table 插槽模板闭包类型定义（WPF DataTemplate 等价）
//!
//! 声明式插槽模板（`<template slot="header/cell/footer">`）由 codegen 转换为
//! 闭包注入到 Table 的对应字段。闭包类型使用 `Arc<dyn Fn ... + Send + Sync>`，
//! 满足 IModel 的 Send + Sync 约束（参考 `rml_core::slot::SlotRenderer`）。
//!
//! # 闭包参数
//!
//! - `HeaderTemplate`: `(col_idx, column, cx) -> AnyElement`
//! - `CellTemplate`: `(row_idx, col_idx, row_data, column, cx) -> AnyElement`
//! - `FooterTemplate`: `(cx) -> AnyElement`
//!
//! # 限制
//!
//! 闭包是 `move`，不捕获父视图 `self`（生命周期限制，与 `user_component.rs` 一致）。
//! 需事件处理时用 `TableDelegate` trait。

use std::sync::Arc;

use gpui::{AnyElement, App};

use super::table_column::TableColumn;
use super::table_row::TableRow;

/// 列头模板闭包（WPF HeaderTemplate 等价）
///
/// 参数：`(col_idx, column, cx) -> AnyElement`
pub type HeaderTemplate =
    Arc<dyn Fn(usize, &TableColumn, &mut App) -> AnyElement + Send + Sync + 'static>;

/// 单元格模板闭包（WPF CellTemplate 等价）
///
/// 参数：`(row_idx, col_idx, row_data, column, cx) -> AnyElement`
pub type CellTemplate = Arc<
    dyn Fn(usize, usize, &TableRow, &TableColumn, &mut App) -> AnyElement + Send + Sync + 'static,
>;

/// 底部模板闭包
///
/// 参数：`(cx) -> AnyElement`
pub type FooterTemplate =
    Arc<dyn Fn(&mut App) -> AnyElement + Send + Sync + 'static>;

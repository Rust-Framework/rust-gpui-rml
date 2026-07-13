//! Table 组件 —— WPF DataGrid 风格的声明式表格
//!
//! 基于 `gpui::div()` + flex 布局模拟表格结构，提供：
//! - `columns` / `rows`：数据绑定式列定义和行数据
//! - `column()`：声明式 `<Column>` 子标签追加列
//! - `delegate`：TableDelegate trait 模板委托（自定义渲染，支持事件处理）
//! - `header_template` / `cell_template` / `footer_template`：声明式插槽模板
//!   （`<template slot="header/cell/footer">`，WPF DataTemplate 等价）
//! - `bordered` / `borderless` / `stripe`：边框和斑马纹样式
//! - `size`：尺寸变体（通过 Sizable trait）
//!
//! 渲染优先级：`cell_templates`（声明式插槽）> `delegate` > `DefaultTableDelegate`
//!
//! RML `<Table>` / `<table>` 编译为 `rml_ui::Table::new(...).<setters>...`：
//! - `columns={expr}` → `.columns(self.expr.clone())`
//! - `rows={expr}` → `.rows(self.expr.clone())`
//! - `delegate={expr}` → `.delegate(self.expr.clone())`（Arc<dyn TableDelegate>）
//! - `bordered=""` → `.bordered(true)` / `stripe=""` → `.stripe(true)`
//! - `<Column key="..." title="..." />` 子标签 → `.column(TableColumn::new(...))`
//! - `<template slot="header">` → `.header_template(Arc::new(...))`
//! - `<template slot="cell" field="key">` → `.cell_template("key", Arc::new(...))`
//! - `<template slot="footer">` → `.footer_template(Arc::new(...))`

use std::collections::HashMap;
use std::sync::Arc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, AnyElement, App, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TextAlign, Window,
};
use gpui_component::{ActiveTheme, Sizable, Size, StyledExt};

/// 单元格编辑提交回调类型
/// `Fn(row, col, new_value, app)`
pub type CellEditHandler = Arc<dyn Fn(usize, usize, SharedString, &mut App) + Send + Sync>;

use super::table_column::TableColumn;
use super::table_delegate::{DefaultTableDelegate, TableDelegate};
use super::table_row::TableRow;
use super::table_template::{CellTemplate, FooterTemplate, HeaderTemplate};

/// 重新渲染通知回调类型
/// `Fn(&mut App)` —— 调用时触发 ViewModel 重新渲染
pub type NotifyCallback = Arc<dyn Fn(&mut App) + Send + Sync>;

/// Table 组件 —— WPF DataGrid 风格的声明式表格
#[derive(IntoElement)]
pub struct Table {
    base: Div,
    id: ElementId,
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    delegate: Option<Arc<dyn TableDelegate>>,
    header_template: Option<HeaderTemplate>,
    cell_templates: HashMap<SharedString, CellTemplate>,
    footer_template: Option<FooterTemplate>,
    on_cell_edit: Option<CellEditHandler>,
    notify: Option<NotifyCallback>,
    bordered: bool,
    stripe: bool,
    size: Size,
}

impl Table {
    /// 创建表格。`id` 由 codegen 自动注入 `("rml_el", N)`。
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            columns: Vec::new(),
            rows: Vec::new(),
            delegate: None,
            header_template: None,
            cell_templates: HashMap::new(),
            footer_template: None,
            on_cell_edit: None,
            notify: None,
            bordered: true,
            stripe: false,
            size: Size::default(),
        }
    }

    /// 数据绑定式列定义（与 `<Column>` 子标签可混用，声明式追加到尾部）。
    pub fn columns(mut self, columns: Vec<TableColumn>) -> Self {
        self.columns.extend(columns);
        self
    }

    /// 行数据绑定。
    pub fn rows(mut self, rows: Vec<TableRow>) -> Self {
        self.rows = rows;
        self
    }

    /// 声明式 Column 子标签追加（codegen 生成 `.column(...)` 调用）。
    pub fn column(mut self, column: TableColumn) -> Self {
        self.columns.push(column);
        self
    }

    /// 模板委托（自定义渲染，用于复杂事件处理场景）。
    ///
    /// 用户在 .rml.rs 中持有 `Arc<dyn TableDelegate>` 字段，
    /// codegen 生成 `.delegate(self.field.clone())`（Arc clone 廉价）。
    pub fn delegate(mut self, delegate: Arc<dyn TableDelegate>) -> Self {
        self.delegate = Some(delegate);
        self
    }

    /// 声明式插槽：列头模板（codegen 从 `<template slot="header">` 生成）。
    pub fn header_template(mut self, template: HeaderTemplate) -> Self {
        self.header_template = Some(template);
        self
    }

    /// 声明式插槽：单元格模板（codegen 从 `<template slot="cell" field="key">` 生成）。
    /// `field` 参数指定列 key，仅对该列应用模板。
    pub fn cell_template(mut self, field: impl Into<SharedString>, template: CellTemplate) -> Self {
        self.cell_templates.insert(field.into(), template);
        self
    }

    /// 声明式插槽：底部模板（codegen 从 `<template slot="footer">` 生成）。
    pub fn footer_template(mut self, template: FooterTemplate) -> Self {
        self.footer_template = Some(template);
        self
    }

    /// 显式控制边框。`true` 显示边框，`false` 隐藏。
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// 无边框变体。
    pub fn borderless(mut self) -> Self {
        self.bordered = false;
        self
    }

    /// 斑马纹样式（奇数行交替背景色）。
    pub fn stripe(mut self, stripe: bool) -> Self {
        self.stripe = stripe;
        self
    }

    /// 单元格编辑提交回调。
    ///
    /// 当可编辑列的单元格编辑完成时调用，回调签名为 `Fn(row, col, new_value, app)`。
    /// 由 codegen 从 `on-cell-edit={handler}` 生成，使用 `cx.weak_entity()` 桥接回 ViewModel。
    pub fn on_cell_edit(mut self, handler: CellEditHandler) -> Self {
        self.on_cell_edit = Some(handler);
        self
    }

    /// 设置重新渲染通知回调。由 codegen 自动生成，使用 `cx.weak_entity()` 桥接。
    /// Table 在 render 时将此回调注入 delegate，使 delegate 能在编辑状态变更时触发重新渲染。
    pub fn notify(mut self, notify: NotifyCallback) -> Self {
        self.notify = Some(notify);
        self
    }
}

impl Styled for Table {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Table {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Table {}

impl Sizable for Table {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Table {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (border_color, header_bg, stripe_bg, radius) = {
            let theme = cx.theme();
            (
                theme.border,
                theme.table_head,
                theme.table_even,
                theme.radius,
            )
        };

        let (text_size, cell_px, cell_py) = match self.size {
            Size::Small => (px(12.), px(8.), px(4.)),
            _ => (px(13.), px(12.), px(6.)),
        };

        let bordered = self.bordered;
        let columns = self.columns;
        let rows = self.rows;
        let header_template = self.header_template;
        let cell_templates = self.cell_templates;
        let footer_template = self.footer_template;
        let delegate = self.delegate;
        let notify = self.notify;

        // 将通知回调注入 delegate，使 delegate 在编辑状态变更时能触发重新渲染
        if let Some(d) = &delegate {
            if let Some(n) = &notify {
                d.set_notify(n.clone());
            }
        }

        // 渲染列头行
        let header_row: Option<AnyElement> = if !columns.is_empty() {
            let mut header_cells: Vec<AnyElement> = Vec::with_capacity(columns.len());
            for (col_idx, column) in columns.iter().enumerate() {
                let content: AnyElement = if let Some(tpl) = &header_template {
                    tpl(col_idx, column, cx)
                } else if let Some(d) = &delegate {
                    d.render_header(col_idx, column, cx)
                } else {
                    DefaultTableDelegate.render_header(col_idx, column, cx)
                };
                let cell = div()
                    .px(cell_px)
                    .py(cell_py)
                    .text_size(text_size)
                    .font_semibold()
                    .min_w(px(0.))
                    .when_some(column.width, |this, w| this.w(w))
                    .when_some(column.align, |this, align| match align {
                        TextAlign::Center => this.text_center(),
                        TextAlign::Right => this.text_right(),
                        _ => this.text_left(),
                    })
                    .bg(header_bg)
                    .child(content);
                header_cells.push(cell.into_any_element());
            }
            Some(
                div()
                    .flex()
                    .bg(header_bg)
                    .when(bordered, |this| {
                        this.border_b_1().border_color(border_color)
                    })
                    .children(header_cells)
                    .into_any_element(),
            )
        } else {
            None
        };

        // 渲染数据行
        let mut body_rows: Vec<AnyElement> = Vec::with_capacity(rows.len());
        for (row_idx, row_data) in rows.iter().enumerate() {
            let mut cells: Vec<AnyElement> = Vec::with_capacity(columns.len());
            let mut skip_count = 0usize;

            for (col_idx, column) in columns.iter().enumerate() {
                // 处理合并列：跳过被前一个单元格覆盖的列
                if skip_count > 0 {
                    skip_count -= 1;
                    continue;
                }

                // 检查合并列跨度
                let col_span = row_data.col_spans.get(&column.key).copied().unwrap_or(1);
                if col_span > 1 {
                    skip_count = col_span - 1;
                }

                let stripe_bg_val = if self.stripe && row_idx % 2 == 1 {
                    Some(stripe_bg)
                } else {
                    None
                };

                // 判断单元格是否可编辑 / 正在编辑
                let is_editable = delegate
                    .as_ref()
                    .map_or(false, |d| d.can_edit(row_idx, col_idx, column));
                let is_editing =
                    is_editable && delegate.as_ref().unwrap().is_editing(row_idx, col_idx);

                // 渲染单元格内容
                let content: AnyElement = if is_editing {
                    delegate
                        .as_ref()
                        .unwrap()
                        .render_editor(row_idx, col_idx, column, row_data, window, cx)
                } else if let Some(tpl) = cell_templates.get(&column.key) {
                    tpl(row_idx, col_idx, row_data, column, cx)
                } else if let Some(d) = &delegate {
                    d.render_cell(row_idx, col_idx, column, row_data, cx)
                } else {
                    DefaultTableDelegate.render_cell(row_idx, col_idx, column, row_data, cx)
                };

                // 构建单元格容器：可编辑且非编辑态时添加点击进入编辑
                let cell: AnyElement = if is_editable && !is_editing {
                    let d = delegate.as_ref().unwrap().clone();
                    let n = notify.clone();
                    div()
                        .id(("editable_cell", row_idx * 10000 + col_idx))
                        .on_click(move |_, _window, cx| {
                            d.start_edit(row_idx, col_idx);
                            if let Some(n) = &n {
                                n(cx);
                            }
                        })
                        .px(cell_px)
                        .py(cell_py)
                        .text_size(text_size)
                        .min_w(px(0.))
                        .when_some(column.width, |this, w| {
                            if col_span > 1 {
                                this.w(w * col_span as f32)
                            } else {
                                this.w(w)
                            }
                        })
                        .when(col_span > 1 && column.width.is_none(), |this| {
                            this.flex_grow(col_span as f32)
                        })
                        .when(col_span <= 1 && column.width.is_none(), |this| {
                            this.flex_1()
                        })
                        .when_some(column.align, |this, align| match align {
                            TextAlign::Center => this.text_center(),
                            TextAlign::Right => this.text_right(),
                            _ => this.text_left(),
                        })
                        .when_some(stripe_bg_val, |this, bg| this.bg(bg))
                        .child(content)
                        .into_any_element()
                } else {
                    div()
                        .px(cell_px)
                        .py(cell_py)
                        .text_size(text_size)
                        .min_w(px(0.))
                        .when_some(column.width, |this, w| {
                            if col_span > 1 {
                                this.w(w * col_span as f32)
                            } else {
                                this.w(w)
                            }
                        })
                        .when(col_span > 1 && column.width.is_none(), |this| {
                            this.flex_grow(col_span as f32)
                        })
                        .when(col_span <= 1 && column.width.is_none(), |this| {
                            this.flex_1()
                        })
                        .when_some(column.align, |this, align| match align {
                            TextAlign::Center => this.text_center(),
                            TextAlign::Right => this.text_right(),
                            _ => this.text_left(),
                        })
                        .when_some(stripe_bg_val, |this, bg| this.bg(bg))
                        .child(content)
                        .into_any_element()
                };
                cells.push(cell);
            }

            body_rows.push(
                div()
                    .flex()
                    .when(bordered, |this| {
                        this.border_b_1().border_color(border_color)
                    })
                    .children(cells)
                    .into_any_element(),
            );
        }

        // 渲染底部
        let footer: Option<AnyElement> = footer_template.as_ref().map(|tpl| {
            div()
                .flex()
                .when(bordered, |this| {
                    this.border_t_1().border_color(border_color)
                })
                .child(tpl(cx))
                .into_any_element()
        });

        self.base
            .id(self.id)
            .flex()
            .flex_col()
            .w_full()
            .overflow_hidden()
            .when(bordered, |this| {
                this.border_1().border_color(border_color).rounded(radius)
            })
            .text_size(text_size)
            .when_some(header_row, |this, h| this.child(h))
            .children(body_rows)
            .when_some(footer, |this, f| this.child(f))
    }
}

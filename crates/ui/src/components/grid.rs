//! Grid / GridItem —— 声明式 Grid 布局
//!
//! RML `<Grid columns="3" rows="2">` 创建等宽网格布局容器。
//! `<GridItem col-span="2" row-start="1">` 控制子项在网格中的位置。
//!
//! 底层使用 GPUI 的 CSS Grid 支持（`div().grid().grid_cols(n).grid_rows(n)`）。
//! GPUI 的 Grid 仅支持等宽列/等高行，不支持任意 `200px 1fr 300px` 模板。
//! 需要非等宽布局时，请使用 `<Resizable>` + `<ResizablePanel>`。

use gpui::{
    div, AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
};
use gpui_component::StyledExt as _;

/// 声明式 Grid 布局容器
///
/// 通过 `columns` / `rows` 属性设置等宽列数和等高行数。
/// 子节点通过 `.child()` 注入，配合 `<GridItem>` 控制每个子项的跨列/跨行。
#[derive(IntoElement)]
pub struct Grid {
    columns: Option<u16>,
    rows: Option<u16>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            columns: None,
            rows: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
}

impl Grid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置等宽列数
    pub fn columns(mut self, cols: u16) -> Self {
        self.columns = Some(cols);
        self
    }

    /// 设置等高行数
    pub fn rows(mut self, rows: u16) -> Self {
        self.rows = Some(rows);
        self
    }
}

impl Styled for Grid {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Grid {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Grid {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut grid = div().grid();
        if let Some(cols) = self.columns {
            grid = grid.grid_cols(cols);
        }
        if let Some(rows) = self.rows {
            grid = grid.grid_rows(rows);
        }
        grid = grid.refine_style(&self.style);
        for child in self.children {
            grid = grid.child(child);
        }
        grid
    }
}

/// Grid 子项，控制元素在 Grid 中的位置和跨度
///
/// 通过 `col-span` / `row-span` / `col-start` / `col-end` / `row-start` / `row-end`
/// 属性控制子项在父 Grid 中的位置。
#[derive(IntoElement)]
pub struct GridItem {
    col_span: Option<u16>,
    row_span: Option<u16>,
    col_start: Option<i16>,
    col_end: Option<i16>,
    row_start: Option<i16>,
    row_end: Option<i16>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Default for GridItem {
    fn default() -> Self {
        Self {
            col_span: None,
            row_span: None,
            col_start: None,
            col_end: None,
            row_start: None,
            row_end: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
}

impl GridItem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn col_span(mut self, span: u16) -> Self {
        self.col_span = Some(span);
        self
    }

    pub fn row_span(mut self, span: u16) -> Self {
        self.row_span = Some(span);
        self
    }

    pub fn col_start(mut self, start: i16) -> Self {
        self.col_start = Some(start);
        self
    }

    pub fn col_end(mut self, end: i16) -> Self {
        self.col_end = Some(end);
        self
    }

    pub fn row_start(mut self, start: i16) -> Self {
        self.row_start = Some(start);
        self
    }

    pub fn row_end(mut self, end: i16) -> Self {
        self.row_end = Some(end);
        self
    }
}

impl Styled for GridItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for GridItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for GridItem {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut item = div();
        if let Some(span) = self.col_span {
            item = item.col_span(span);
        }
        if let Some(span) = self.row_span {
            item = item.row_span(span);
        }
        if let Some(start) = self.col_start {
            item = item.col_start(start);
        }
        if let Some(end) = self.col_end {
            item = item.col_end(end);
        }
        if let Some(start) = self.row_start {
            item = item.row_start(start);
        }
        if let Some(end) = self.row_end {
            item = item.row_end(end);
        }
        item = item.refine_style(&self.style);
        for child in self.children {
            item = item.child(child);
        }
        item
    }
}

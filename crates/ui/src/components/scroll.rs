//! Scroll —— 声明式滚动容器
//!
//! RML `<Scroll>` 标签编译为 `Scroll` 组件，封装 `div().overflow_y_scrollbar()` 模式。
//!
//! ## 设计原因
//!
//! gpui-component 的 `ScrollableElement` trait 提供 `.overflow_y_scrollbar()` / `.overflow_x_scrollbar()`
//! / `.overflow_scrollbar()` 方法，返回 `Scrollable<Div>` 包装器（自带滚动条 UI）。
//! RML 需要声明式容器组件，通过 `axis` 属性选择滚动方向。
//!
//! ## 构造模式
//!
//! ```ignore
//! Scroll::new()
//!     .vertical()  // 或 .horizontal() 或 .both()，默认 vertical
//!     .child(...)
//! ```

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
};
use gpui_component::scroll::{ScrollableElement as _, ScrollbarAxis};
use gpui_component::StyledExt as _;

/// 声明式滚动容器
///
/// 通过 `axis` 属性选择滚动方向（vertical/horizontal/both），子节点通过 `.child()` 注入。
/// 底层使用 gpui-component 的 `Scrollable<Div>` 包装器，自带滚动条 UI。
#[derive(IntoElement)]
pub struct Scroll {
    axis: ScrollbarAxis,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Default for Scroll {
    fn default() -> Self {
        Self::new()
    }
}

impl Scroll {
    /// 创建滚动容器，默认垂直滚动
    pub fn new() -> Self {
        Self {
            axis: ScrollbarAxis::Vertical,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// 设置为垂直滚动（默认）
    pub fn vertical(mut self) -> Self {
        self.axis = ScrollbarAxis::Vertical;
        self
    }

    /// 设置为水平滚动
    pub fn horizontal(mut self) -> Self {
        self.axis = ScrollbarAxis::Horizontal;
        self
    }

    /// 设置为双向滚动（垂直 + 水平）
    pub fn both(mut self) -> Self {
        self.axis = ScrollbarAxis::Both;
        self
    }
}

impl Styled for Scroll {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Scroll {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Scroll {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut scroll = match self.axis {
            ScrollbarAxis::Vertical => gpui::div().overflow_y_scrollbar(),
            ScrollbarAxis::Horizontal => gpui::div().overflow_x_scrollbar(),
            ScrollbarAxis::Both => gpui::div().overflow_scrollbar(),
        };
        scroll = scroll.refine_style(&self.style);
        for child in self.children {
            scroll = scroll.child(child);
        }
        scroll
    }
}

//! `StatusBar` —— 状态栏对齐枚举 + 框架 `NativeStatusBar`。
//!
//! `StatusBarAlign` 已移至 `rml_core::contribution`(因 `IStatusBarItem::align()` 返回类型
//! 需要在 core 定义)。本模块经 `pub use` re-export 保持 `rml_ui::StatusBarAlign` 兼容。
//!
//! 框架提供 `IStatusBarItem` trait(仅 `align()`)作为状态栏容器的数据契约。
//! 业务侧 `MainWindow::render_status_bar()` 经 `NativeStatusBar::new()` + `.left()` / `.right()`
//! / `.child()` 组装,对齐信息由 `IStatusBarItem::align()` 提取。
//!
//! `NativeStatusBar` 基于 gpui-component `StatusBar` 布局，去掉顶部边框以与 chrome 层视觉融合。

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
    prelude::FluentBuilder as _,
};
use gpui_component::{ActiveTheme, StyledExt, h_flex};
use smallvec::SmallVec;

pub use rml_core::contribution::StatusBarAlign;

/// 原生状态栏 —— 左/中/右三区布局，无顶部边框。
#[derive(IntoElement)]
pub struct NativeStatusBar {
    style: StyleRefinement,
    left: SmallVec<[AnyElement; 1]>,
    right: SmallVec<[AnyElement; 1]>,
    children: SmallVec<[AnyElement; 1]>,
}

impl NativeStatusBar {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            left: SmallVec::new(),
            right: SmallVec::new(),
            children: SmallVec::new(),
        }
    }

    pub fn left(mut self, child: impl IntoElement) -> Self {
        self.left.push(child.into_any_element());
        self
    }

    pub fn right(mut self, child: impl IntoElement) -> Self {
        self.right.push(child.into_any_element());
        self
    }
}

impl ParentElement for NativeStatusBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for NativeStatusBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NativeStatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let has_left = !self.left.is_empty();
        let has_right = !self.right.is_empty();
        let region = || h_flex().overflow_hidden().items_center().gap_2();

        h_flex()
            .items_center()
            .gap_2()
            .py_1()
            .px_2()
            .bg(cx.theme().tokens.status_bar)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .when(has_left, |this| this.child(region().children(self.left)))
            .child(
                region()
                    .flex_1()
                    .when(has_left && has_right, |this| this.justify_center())
                    .when(has_left && !has_right, |this| this.justify_end())
                    .children(self.children),
            )
            .when(has_right, |this| this.child(region().children(self.right)))
    }
}

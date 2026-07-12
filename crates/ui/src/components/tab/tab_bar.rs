//! 原生 gpui-component 形态的 TabBar —— 纯 header 标签栏（无 body/无 close/无 promote）。
//!
//! 与 [`super::Tabs`] 的关键差异：
//! - `Tabs`：WPF TabControl 风格，header + body 切换，支持 on_close/on_promote/bordered
//! - `TabBar`：纯 header 标签栏，仅支持 on_click 选中切换，无 body 概念
//!
//! 实现策略：内部委托 [`super::Tabs`] 渲染。当所有 TabItem 的 body=None 时，Tabs 自动
//! 退化为 header-only 渲染（不堆叠 v_flex body）。TabBar 仅暴露 header 相关 API，
//! 不暴露 body/close/promote/bordered 方法，呈现原生 TabBar 形态。

use gpui::{
    App, ElementId, IntoElement, RenderOnce, ScrollHandle, StyleRefinement, Styled, Window,
};
use gpui_component::{Sizable, Size};

use super::{TabItem, TabVariant, Tabs};

/// 原生 gpui-component 形态的 TabBar —— 纯 header 标签栏。
///
/// 详见模块级文档。
#[derive(IntoElement)]
pub struct TabBar {
    inner: Tabs,
}

impl TabBar {
    /// 创建一个新 TabBar。
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            inner: Tabs::new(id),
        }
    }

    /// Set the Tab variant, all children will inherit the variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.inner = self.inner.with_variant(variant);
        self
    }

    /// Set the Tab variant to Pill, all children will inherit the variant.
    pub fn pill(mut self) -> Self {
        self.inner = self.inner.pill();
        self
    }

    /// Set the Tab variant to Flat, all children will inherit the variant.
    pub fn flat(mut self) -> Self {
        self.inner = self.inner.flat();
        self
    }

    /// Set the Tab variant to Outline, all children will inherit the variant.
    pub fn outline(mut self) -> Self {
        self.inner = self.inner.outline();
        self
    }

    /// Set the Tab variant to Segmented, all children will inherit the variant.
    pub fn segmented(mut self) -> Self {
        self.inner = self.inner.segmented();
        self
    }

    /// Set the Tab variant to Underline, all children will inherit the variant.
    pub fn underline(mut self) -> Self {
        self.inner = self.inner.underline();
        self
    }

    /// Set whether to show the menu button when tabs overflow, default is false.
    pub fn menu(mut self, menu: bool) -> Self {
        self.inner = self.inner.menu(menu);
        self
    }

    /// When true, draw a separator under the tab strip and merge the selected tab
    /// with a body panel placed below this header-only TabBar.
    pub fn connect_body(mut self, connect_body: bool) -> Self {
        self.inner = self.inner.connect_body(connect_body);
        self
    }

    /// Track the scroll of the TabBar.
    pub fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.inner = self.inner.track_scroll(scroll_handle);
        self
    }

    /// Set the prefix element of the TabBar.
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.inner = self.inner.prefix(prefix);
        self
    }

    /// Set the suffix element of the TabBar.
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.inner = self.inner.suffix(suffix);
        self
    }

    /// Add children of the TabBar, all children will inherit the variant.
    ///
    /// 接受 `impl Into<TabItem>`，兼容 `Tab`（通过 `From<Tab> for TabItem` 转换）和 `TabItem`。
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<TabItem>>) -> Self {
        self.inner = self.inner.children(children);
        self
    }

    /// Add child of the TabBar, tab will inherit the variant.
    ///
    /// 接受 `impl Into<TabItem>`，兼容 `Tab`（通过 `From<Tab> for TabItem` 转换）和 `TabItem`。
    pub fn child(mut self, child: impl Into<TabItem>) -> Self {
        self.inner = self.inner.child(child);
        self
    }

    /// Set the selected index of the TabBar.
    pub fn selected_index(mut self, index: usize) -> Self {
        self.inner = self.inner.selected_index(index);
        self
    }

    /// Set the last empty space element of the TabBar.
    pub fn last_empty_space(mut self, last_empty_space: impl IntoElement) -> Self {
        self.inner = self.inner.last_empty_space(last_empty_space);
        self
    }

    /// Set the on_click callback of the TabBar, the first parameter is the index of the clicked tab.
    ///
    /// When this is set, the children's on_click will be ignored.
    pub fn on_click<F>(mut self, on_click: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.inner = self.inner.on_click(on_click);
        self
    }
}

impl Styled for TabBar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl Sizable for TabBar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.inner
    }
}

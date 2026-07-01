//! `StatusBar` —— RML MVVM 状态栏（gpui-component 无 items 绑定，由本 crate 定义）
//!
//! 包装 gpui-component [`NativeStatusBar`]，提供 `IStatusBarItem` + `items={...}` 绑定。
//! 手动 `.left()` / `.right()` 组装请使用 [`NativeStatusBar`]（RML 标签 `<NativeStatusBar>`）。

use std::sync::Arc;

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Window};

pub use gpui_component::status_bar::StatusBar as NativeStatusBar;

/// 状态栏项对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarAlign {
    Left,
    Right,
    Center,
}

/// 状态栏项接口（object-safe）
pub trait IStatusBarItem: Send + Sync + 'static {
    fn content(&self) -> SharedString;
    fn align(&self) -> StatusBarAlign {
        StatusBarAlign::Left
    }
}

/// 状态栏项列表类型别名
pub type StatusBarItems = Vec<Arc<dyn IStatusBarItem>>;

/// 状态栏项默认实现
pub struct StatusBarItem {
    content: SharedString,
    align: StatusBarAlign,
}

impl StatusBarItem {
    pub fn new(content: impl Into<SharedString>) -> Self {
        Self {
            content: content.into(),
            align: StatusBarAlign::Left,
        }
    }

    pub fn align(mut self, a: StatusBarAlign) -> Self {
        self.align = a;
        self
    }

    pub fn into_arc(self) -> Arc<dyn IStatusBarItem> {
        Arc::new(self)
    }
}

impl IStatusBarItem for StatusBarItem {
    fn content(&self) -> SharedString {
        self.content.clone()
    }

    fn align(&self) -> StatusBarAlign {
        self.align
    }
}

/// RML 状态栏（`<status_bar items={...}>`）
#[derive(IntoElement)]
pub struct StatusBar {
    items: StatusBarItems,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn items(mut self, items: StatusBarItems) -> Self {
        self.items = items;
        self
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut left_items: Vec<SharedString> = Vec::new();
        let mut center_items: Vec<SharedString> = Vec::new();
        let mut right_items: Vec<SharedString> = Vec::new();

        for item in &self.items {
            match item.align() {
                StatusBarAlign::Left => left_items.push(item.content()),
                StatusBarAlign::Center => center_items.push(item.content()),
                StatusBarAlign::Right => right_items.push(item.content()),
            }
        }

        let mut bar = NativeStatusBar::new();
        for content in left_items {
            bar = bar.left(content);
        }
        for content in right_items {
            bar = bar.right(content);
        }
        for content in center_items {
            bar = bar.child(content);
        }
        bar
    }
}

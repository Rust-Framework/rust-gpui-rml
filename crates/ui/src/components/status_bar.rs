//! RmlStatusBar —— MVVM 数据绑定的状态栏包装组件
//!
//! 包装 gpui-component `StatusBar`，提供 `IStatusBarItem` trait + `StatusBarItem`
//! 默认实现 + `StatusBarItems = Vec<Arc<dyn IStatusBarItem>>` 类型别名。
//!
//! ViewModel 通过 `#[computed]` 返回 `StatusBarItems`，
//! 在 RML 中 `<status_bar items={status_items} />` 绑定。

use std::sync::Arc;

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Window};
use gpui_component::status_bar::StatusBar;

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

/// RML 状态栏包装组件
///
/// StatelessNoId 构造：`RmlStatusBar::new().items(items)`
#[derive(IntoElement)]
pub struct RmlStatusBar {
    items: StatusBarItems,
}

impl RmlStatusBar {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn items(mut self, items: StatusBarItems) -> Self {
        self.items = items;
        self
    }
}

impl Default for RmlStatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for RmlStatusBar {
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

        let mut bar = StatusBar::new();
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

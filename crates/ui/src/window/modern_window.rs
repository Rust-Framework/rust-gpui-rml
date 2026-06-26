//! ModernWindowShell —— 内置封装的窗口视觉外壳组件
//!
//! 组合 `TitleBar` + `Menu` + `StatusBar`，用户通过 MVVM 数据绑定配置，
//! 无需在 `.rml` 中编写 `<TitleBar><Menu>...</Menu></TitleBar>` 布局。
//!
//! 用户也可选择手动组装：用 `<TitleBar>` / `<StatusBar>` / `<Kbd>` 原子组件自行构建。
//! ModernWindowShell 是易用性封装，基于它构建的 `.rml` 文件代码更少，更符合现代视觉应用设计。
//!
//! 注：重命名自 `ModernWindow`，以释放该名称给 `IWindow` 实现使用。

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_component::{TitleBar, status_bar::StatusBar};
use smallvec::SmallVec;

use super::menu_bar::render_menu_bar;
use super::types::{MenuItem, StatusBarItem};

/// ModernWindowShell —— 内置封装 TitleBar + Menu + StatusBar 的 RenderOnce 组件
///
/// 在 `.rml` 中作为根标签使用：
/// ```html
/// <ModernWindowShell title="My App" menu={menu_items} status_bar={status_items}>
///     <!-- 业务内容 -->
/// </ModernWindowShell>
/// ```
#[derive(IntoElement)]
pub struct ModernWindowShell {
    title: Option<gpui::SharedString>,
    menu: Option<Vec<MenuItem>>,
    status_bar: Option<Vec<StatusBarItem>>,
    children: SmallVec<[AnyElement; 4]>,
}

impl ModernWindowShell {
    pub fn new() -> Self {
        Self {
            title: None,
            menu: None,
            status_bar: None,
            children: SmallVec::new(),
        }
    }

    /// 绑定标题栏内容（MVVM 数据绑定入口）
    pub fn title(mut self, title: impl Into<gpui::SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 绑定菜单数据（MVVM 数据绑定入口）
    ///
    /// ViewModel 持有 `Vec<MenuItem>` 字段，在 RML 中 `menu={self.menu_items}`
    pub fn menu(mut self, menu: Vec<MenuItem>) -> Self {
        self.menu = Some(menu);
        self
    }

    /// 绑定状态栏数据（MVVM 数据绑定入口）
    pub fn status_bar(mut self, items: Vec<StatusBarItem>) -> Self {
        self.status_bar = Some(items);
        self
    }
}

impl Default for ModernWindowShell {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for ModernWindowShell {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ModernWindowShell {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                TitleBar::new()
                    .when_some(self.title, |this, title| this.child(title))
                    .when_some(self.menu, |this, menu| {
                        this.child(render_menu_bar(&menu))
                    }),
            )
            .children(self.children)
            .when_some(self.status_bar, |this, items: Vec<StatusBarItem>| {
                this.child(render_status_bar(&items))
            })
    }
}

fn render_status_bar(items: &[StatusBarItem]) -> impl IntoElement {
    items.iter().fold(StatusBar::new(), |bar, item| {
        bar.left(item.label.clone())
    })
}

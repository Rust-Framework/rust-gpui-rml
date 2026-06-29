//! ModernWindowShell —— 内置封装的窗口视觉外壳组件
//!
//! 组合 `TitleBar` + `Menu` + `StatusBar`，用户通过 MVVM 数据绑定配置，
//! 无需在 `.rml` 中编写 `<TitleBar><Menu>...</Menu></TitleBar>` 布局。
//!
//! ## 标题栏布局
//!
//! 默认从左到右排列：
//! ```text
//! [图标][主窗口菜单] ... [窗口标题居中占满] ... [可扩展] [窗口操作]
//! |<------- 左对齐 ------>| |<------ 居中 ------>| |<-- 右对齐 -->| (TitleBar 内置)
//! ```
//! - 图标 + 主窗口菜单：左对齐
//! - 窗口标题：占满剩余空间，文本水平居中
//! - 可扩展区域：右对齐（位于窗口操作按钮左侧）
//! - 窗口操作按钮（最小化/最大化/关闭）：由 `TitleBar` 内置在最右侧

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_component::{Icon, IconName, Sizable as _, TitleBar, h_flex, status_bar::StatusBar};
use smallvec::SmallVec;

use super::menu_bar::render_menu_bar;
use super::types::{MenuItem, StatusBarItem};

/// ModernWindowShell —— 内置封装 TitleBar + Menu + StatusBar 的 RenderOnce 组件
///
/// 在 `.rml` 中作为根标签使用：
/// ```html
/// <modern_window title="My App" icon={IconName::Frame} menu={menu_items} status_bar={status_items}>
///     <!-- 业务内容 -->
/// </modern_window>
/// ```
#[derive(IntoElement)]
pub struct ModernWindowShell {
    title: Option<gpui::SharedString>,
    icon: Option<IconName>,
    menu: Option<Vec<MenuItem>>,
    extensible: Option<AnyElement>,
    status_bar: Option<Vec<StatusBarItem>>,
    children: SmallVec<[AnyElement; 4]>,
}

impl ModernWindowShell {
    pub fn new() -> Self {
        Self {
            title: None,
            icon: None,
            menu: None,
            extensible: None,
            status_bar: None,
            children: SmallVec::new(),
        }
    }

    /// 绑定窗口标题（MVVM 数据绑定入口）
    ///
    /// 标题在标题栏中占满剩余空间，文本水平居中。
    pub fn title(mut self, title: impl Into<gpui::SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 绑定窗口图标（MVVM 数据绑定入口）
    ///
    /// 图标位于标题栏最左侧，主窗口菜单的左边。
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 绑定菜单数据（MVVM 数据绑定入口）
    ///
    /// ViewModel 持有 `Vec<MenuItem>` 字段，在 RML 中 `menu={self.menu_items}`
    pub fn menu(mut self, menu: Vec<MenuItem>) -> Self {
        self.menu = Some(menu);
        self
    }

    /// 绑定可扩展区域（标题栏右侧、窗口操作按钮左侧）
    ///
    /// 用于放置自定义工具按钮、Kbd 快捷键提示等。
    pub fn extensible(mut self, element: impl IntoElement) -> Self {
        self.extensible = Some(element.into_any_element());
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
                TitleBar::new().child(
                    h_flex()
                        .flex_1()
                        .h_full()
                        .items_center()
                        // 左侧：图标 + 主窗口菜单（左对齐）
                        .when_some(self.icon, |this, icon| {
                            this.child(h_flex().items_center().pl_2().child(Icon::new(icon).small()))
                        })
                        .when_some(self.menu, |this, menu| {
                            this.child(render_menu_bar(&menu))
                        })
                        // 中间：窗口标题占满剩余空间，文本水平居中
                        .child(
                            div()
                                .flex_1()
                                .text_center()
                                .when_some(self.title, |this, title| this.child(title)),
                        )
                        // 右侧：可扩展区域（WindowOps 由 TitleBar 自动渲染在最右）
                        .when_some(self.extensible, |this, ext| this.child(ext)),
                ),
            )
            // 业务内容包裹在 flex-1 容器中占据剩余空间，
            // 使 StatusBar 自然贴底（TitleBar 在顶，StatusBar 在底，中间内容 flex-1 填充）
            .child(div().flex_1().min_h_0().children(self.children))
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

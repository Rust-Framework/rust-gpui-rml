//! ModernWindowShell —— 内置封装的窗口视觉外壳组件
//!
//! 组合 `TitleBar` + 插槽化 Menu/StatusBar，用户通过 MVVM 数据绑定或自定义 element 配置。

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_component::{Icon, IconName, Sizable as _, TitleBar, h_flex};
use smallvec::SmallVec;

use super::templates::{MenuBarTemplate, StatusBarTemplate};
use super::types::{MenuItem, StatusBarItem};

/// ModernWindowShell —— 内置封装 TitleBar + 插槽 + StatusBar 的 RenderOnce 组件
#[derive(IntoElement)]
pub struct ModernWindowShell {
    title: Option<gpui::SharedString>,
    icon: Option<IconName>,
    show_chrome: bool,
    menu_slot: Option<AnyElement>,
    title_ext_slot: Option<AnyElement>,
    status_slot: Option<AnyElement>,
    children: SmallVec<[AnyElement; 4]>,
}

impl ModernWindowShell {
    pub fn new() -> Self {
        Self {
            title: None,
            icon: None,
            show_chrome: true,
            menu_slot: None,
            title_ext_slot: None,
            status_slot: None,
            children: SmallVec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<gpui::SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 是否显示菜单与标题区域
    pub fn show_chrome(mut self, show: bool) -> Self {
        self.show_chrome = show;
        self
    }

    /// 标题栏菜单插槽
    pub fn menu_slot(mut self, element: impl IntoElement) -> Self {
        self.menu_slot = Some(element.into_any_element());
        self
    }

    /// 兼容：绑定 `Vec<MenuItem>` 菜单数据
    pub fn menu(mut self, menu: Vec<MenuItem>) -> Self {
        self.menu_slot = Some(MenuBarTemplate::new(menu).into_any_element());
        self
    }

    /// 标题栏右侧扩展插槽
    pub fn title_ext_slot(mut self, element: impl IntoElement) -> Self {
        self.title_ext_slot = Some(element.into_any_element());
        self
    }

    /// 兼容：可扩展区域别名
    pub fn extensible(mut self, element: impl IntoElement) -> Self {
        self.title_ext_slot(element)
    }

    /// 底部状态栏插槽
    pub fn status_slot(mut self, element: impl IntoElement) -> Self {
        self.status_slot = Some(element.into_any_element());
        self
    }

    /// 兼容：绑定 `Vec<StatusBarItem>` 状态栏数据
    pub fn status_bar(mut self, items: Vec<StatusBarItem>) -> Self {
        self.status_slot = Some(StatusBarTemplate::new(items).into_any_element());
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
                    .child(
                        h_flex()
                            .flex_1()
                            .h_full()
                            .items_center()
                            .when_some(self.icon, |this, icon| {
                                this.child(h_flex().items_center().pl_2().child(Icon::new(icon).small()))
                            })
                            .when(self.show_chrome, |this| {
                                this.when_some(self.menu_slot, |bar, menu| bar.child(menu))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_center()
                                            .when_some(self.title, |el, title| el.child(title)),
                                    )
                            })
                            .when(!self.show_chrome, |this| this.child(div().flex_1()))
                            .when_some(self.title_ext_slot, |this, ext| this.child(ext)),
                    ),
            )
            .child(div().flex_1().min_h_0().children(self.children))
            .when_some(self.status_slot, |this, slot| this.child(slot))
    }
}

/// 渲染状态栏（供模板复用）
pub(crate) fn render_status_bar(items: &[StatusBarItem]) -> impl IntoElement {
    use gpui_component::status_bar::StatusBar;
    items.iter().fold(StatusBar::new(), |bar, item| {
        bar.left(item.label.clone())
    })
}

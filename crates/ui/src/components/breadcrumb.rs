//! Breadcrumb 面包屑导航组件（VSCode 风格）
//!
//! 水平展示符号路径，项之间以 `›` 分隔。用于编辑器 header 左侧，
//! 对接 LSP documentSymbol 服务显示当前光标位置的符号路径。
//!
//! RML `<Breadcrumb items={breadcrumb_items} />` 编译为
//! `rml_ui::Breadcrumb::new().items(self.breadcrumb_items.clone())`。

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px};
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, Size, h_flex};

/// 面包屑项
#[derive(Clone)]
pub struct BreadcrumbItem {
    pub label: SharedString,
    pub icon: Option<IconName>,
}

impl BreadcrumbItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl From<&str> for BreadcrumbItem {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for BreadcrumbItem {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<SharedString> for BreadcrumbItem {
    fn from(s: SharedString) -> Self {
        Self::new(s)
    }
}

/// 面包屑导航组件
///
/// 水平排列 items，项之间以 `›` 分隔。空 items 时渲染空 div（占位）。
#[derive(IntoElement, Default)]
pub struct Breadcrumb {
    items: Vec<BreadcrumbItem>,
}

impl Breadcrumb {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn items(mut self, items: Vec<BreadcrumbItem>) -> Self {
        self.items = items;
        self
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let total = self.items.len();

        h_flex()
            .items_center()
            .gap_1()
            .text_xs()
            .text_color(theme.muted_foreground)
            .children(
                self.items
                    .into_iter()
                    .enumerate()
                    .flat_map(move |(i, item)| {
                        let is_last = i == total.saturating_sub(1);
                        let mut elements: Vec<gpui::AnyElement> = Vec::new();

                        let mut item_el = h_flex().items_center().gap_1();
                        if let Some(icon) = item.icon {
                            item_el = item_el.child(
                                Icon::new(icon).with_size(Size::Size(px(12.))),
                            );
                        }
                        elements.push(item_el.child(item.label).into_any_element());

                        if !is_last {
                            elements.push(
                                div()
                                    .text_color(theme.border)
                                    .child(SharedString::from("›"))
                                    .into_any_element(),
                            );
                        }
                        elements
                    }),
            )
    }
}

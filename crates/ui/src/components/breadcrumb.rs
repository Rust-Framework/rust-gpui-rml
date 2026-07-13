//! Breadcrumb 面包屑导航组件（VSCode 风格）
//!
//! 水平展示符号路径，项之间以 `›` 分隔。每一级可点击展开下拉选择同级元素，
//! 当前选中项以勾选标记。用于编辑器 header 左侧，
//! 对接 LSP documentSymbol 服务显示当前光标位置的符号路径。
//!
//! RML `<Breadcrumb items={breadcrumb_items} on-select={on_breadcrumb_select} />` 编译为
//! `rml_ui::Breadcrumb::new().items(self.breadcrumb_items.clone())
//!     .on_select_rc(Rc::new(...))`。

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder as _, px, Anchor, App, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};
use gpui_component::{
    h_flex,
    menu::{PopupMenu, PopupMenuItem},
    popover::Popover,
    ActiveTheme, Icon, IconName, Selectable, Sizable, Size,
};

/// 面包屑同级元素（用于下拉列表项）
#[derive(Clone)]
pub struct BreadcrumbSibling {
    pub label: SharedString,
    pub icon: Option<IconName>,
}

impl BreadcrumbSibling {
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

impl From<&str> for BreadcrumbSibling {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for BreadcrumbSibling {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<SharedString> for BreadcrumbSibling {
    fn from(s: SharedString) -> Self {
        Self::new(s)
    }
}

/// 面包屑项（一级）
///
/// `label` 为当前选中项的显示文本，`siblings` 为该级所有可选同级元素，
/// `selected_index` 标识当前选中项在 `siblings` 中的位置（下拉中以勾选标记）。
#[derive(Clone)]
pub struct BreadcrumbItem {
    pub label: SharedString,
    pub icon: Option<IconName>,
    pub siblings: Vec<BreadcrumbSibling>,
    pub selected_index: usize,
}

impl BreadcrumbItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            siblings: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn siblings(mut self, siblings: Vec<BreadcrumbSibling>) -> Self {
        self.siblings = siblings;
        self
    }

    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
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
/// 水平排列 items，项之间以 `›` 分隔。每级点击展开下拉选择同级元素。
/// 空 items 时渲染空 div（占位）。
#[derive(IntoElement, Default)]
pub struct Breadcrumb {
    items: Vec<BreadcrumbItem>,
    on_select: Option<Rc<dyn Fn(usize, usize, &mut Window, &mut App)>>,
}

impl Breadcrumb {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            on_select: None,
        }
    }

    pub fn items(mut self, items: Vec<BreadcrumbItem>) -> Self {
        self.items = items;
        self
    }

    /// 设置同级选择回调：`Fn(level: usize, sibling_index: usize, &mut Window, &mut App)`
    ///
    /// 由 RML 编译器为 `on-select={on_breadcrumb_select}` 生成：
    /// `.on_select_rc(Rc::new({ let weak = cx.weak_entity(); move |level, idx, w, app| { ... } }))`
    pub fn on_select_rc(
        mut self,
        callback: Rc<dyn Fn(usize, usize, &mut Window, &mut App)>,
    ) -> Self {
        self.on_select = Some(callback);
        self
    }
}

/// 内部 trigger 元素：实现 Selectable 以满足 `Popover::trigger` 约束
#[derive(IntoElement)]
struct BreadcrumbTrigger {
    label: SharedString,
    icon: Option<IconName>,
    selected: bool,
}

impl BreadcrumbTrigger {
    fn new(label: SharedString, icon: Option<IconName>) -> Self {
        Self {
            label,
            icon,
            selected: false,
        }
    }
}

impl Selectable for BreadcrumbTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for BreadcrumbTrigger {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .items_center()
            .gap_1()
            .px_1()
            .py_0p5()
            .cursor_pointer()
            .rounded_sm()
            .when(self.selected, |this| this.bg(theme.accent.opacity(0.1)))
            .when_some(self.icon, |this, icon| {
                this.child(Icon::new(icon).with_size(Size::Size(px(12.))))
            })
            .child(self.label)
            .child(
                Icon::new(IconName::ChevronDown)
                    .with_size(Size::Size(px(10.)))
                    .text_color(theme.muted_foreground),
            )
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let total = self.items.len();
        let on_select = self.on_select.clone();

        h_flex()
            .items_center()
            .gap_1()
            .text_xs()
            .text_color(theme.muted_foreground)
            .children(
                self.items
                    .into_iter()
                    .enumerate()
                    .flat_map(move |(level, item)| {
                        let is_last = level == total.saturating_sub(1);
                        let mut elements: Vec<gpui::AnyElement> = Vec::new();

                        let siblings_rc = Rc::new(item.siblings.clone());
                        let on_select_rc = on_select.clone();
                        let selected_index = item.selected_index;
                        let trigger = BreadcrumbTrigger::new(item.label.clone(), item.icon);
                        let popover_id = SharedString::from(format!("breadcrumb:{}", level));

                        let popover = Popover::new(popover_id)
                            .appearance(false)
                            .anchor(Anchor::BottomLeft)
                            .trigger(trigger)
                            .content(move |_, window, cx| {
                                let siblings = siblings_rc.clone();
                                let on_select_clone = on_select_rc.clone();
                                PopupMenu::build(window, cx, move |mut menu, _w, _cx| {
                                    for (idx, sib) in siblings.iter().enumerate() {
                                        let label = sib.label.clone();
                                        let icon = sib.icon.clone();
                                        let cb_clone = on_select_clone.clone();
                                        let mut menu_item = PopupMenuItem::new(label);
                                        if let Some(icon) = icon {
                                            menu_item = menu_item.icon(icon);
                                        }
                                        menu_item = menu_item.checked(idx == selected_index);
                                        menu_item = menu_item.on_click(move |_, w, app| {
                                            if let Some(cb) = cb_clone.as_ref() {
                                                cb(level, idx, w, app);
                                            }
                                        });
                                        menu = menu.item(menu_item);
                                    }
                                    menu
                                })
                                .into_any_element()
                            });

                        elements.push(popover.into_any_element());

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

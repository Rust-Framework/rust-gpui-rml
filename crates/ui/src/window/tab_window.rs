//! TabWindowShell —— TabBar 标题栏 + 可调整插槽的高级窗口壳
//!
//! 布局（单行标题栏）：
//! `[图标切换] [菜单] [标题] [Tab…] [扩展区 suffix] [窗口操作]`
//!
//! 主体插槽：`slot_left` / `slot_right` / `slot_bottom`（可 resize，空则隐藏）、
//! `slot_footer` → `status_slot`（状态栏，空则隐藏）。

use std::rc::Rc;

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, px, prelude::FluentBuilder as _,
};
use gpui_component::{
    Icon, IconName, Sizable as _,
    TitleBar,
    button::{Button, ButtonRounded, ButtonVariants as _},
    tab::{Tab, TabBar},
    h_flex,
    resizable::{h_resizable, resizable_panel, v_resizable},
    v_flex, TITLE_BAR_HEIGHT,
};
use smallvec::SmallVec;

/// Tab 页签数据
#[derive(Clone)]
pub struct TabItem {
    pub label: SharedString,
    pub icon: Option<IconName>,
}

impl TabItem {
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

/// 按标签文案粗算 Tab 总宽度（约 8px/字符 + 48px 内边距与图标余量）
fn estimated_tabs_width(tabs: &[TabItem]) -> gpui::Pixels {
    let total: f32 = tabs
        .iter()
        .map(|tab| tab.label.len() as f32 * 8. + 48.)
        .sum();
    px(total)
}

/// 估算非 Tab 区域占用宽度（prefix、suffix、窗口控件）
fn reserved_title_width(
    show_chrome: bool,
    has_icon: bool,
    has_menu: bool,
    title: Option<&SharedString>,
    has_suffix: bool,
) -> gpui::Pixels {
    let mut reserved = px(140.); // 窗口控件 + TabBar 内边距

    if has_icon {
        reserved += TITLE_BAR_HEIGHT;
    }
    if has_suffix {
        reserved += px(80.);
    }

    if show_chrome {
        if has_menu {
            reserved += px(200.);
        }
        if let Some(title) = title {
            reserved += px(title.len() as f32 * 7. + 24.);
        }
    }

    reserved
}

/// 估算 Tab 是否溢出可用宽度
fn tabs_overflow(tabs: &[TabItem], available_width: gpui::Pixels) -> bool {
    if tabs.is_empty() {
        return false;
    }
    estimated_tabs_width(tabs) > available_width
}

/// TabWindow 高级窗口壳
#[derive(IntoElement)]
pub struct TabWindowShell {
    title: Option<SharedString>,
    icon: Option<IconName>,
    show_chrome: bool,
    menu_slot: Option<AnyElement>,
    title_ext_slot: Option<AnyElement>,
    tabs: Vec<TabItem>,
    selected_tab: usize,
    on_tab_click: Option<Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    on_chrome_toggle: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    slot_left: Option<AnyElement>,
    slot_right: Option<AnyElement>,
    slot_bottom: Option<AnyElement>,
    status_slot: Option<AnyElement>,
    left_width: gpui::Pixels,
    right_width: gpui::Pixels,
    bottom_height: gpui::Pixels,
    children: SmallVec<[AnyElement; 4]>,
}

impl TabWindowShell {
    pub fn new() -> Self {
        Self {
            title: None,
            icon: None,
            show_chrome: true,
            menu_slot: None,
            title_ext_slot: None,
            tabs: Vec::new(),
            selected_tab: 0,
            on_tab_click: None,
            on_chrome_toggle: None,
            slot_left: None,
            slot_right: None,
            slot_bottom: None,
            status_slot: None,
            left_width: px(260.),
            right_width: px(320.),
            bottom_height: px(200.),
            children: SmallVec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn show_chrome(mut self, show: bool) -> Self {
        self.show_chrome = show;
        self
    }

    pub fn menu_slot(mut self, element: impl IntoElement) -> Self {
        self.menu_slot = Some(element.into_any_element());
        self
    }

    pub fn title_ext_slot(mut self, element: impl IntoElement) -> Self {
        self.title_ext_slot = Some(element.into_any_element());
        self
    }

    pub fn tabs(mut self, tabs: Vec<TabItem>) -> Self {
        self.tabs = tabs;
        self
    }

    pub fn selected_tab(mut self, index: usize) -> Self {
        self.selected_tab = index;
        self
    }

    pub fn on_tab_click(
        mut self,
        f: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_tab_click = Some(Rc::new(f));
        self
    }

    pub fn on_chrome_toggle(
        mut self,
        f: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_chrome_toggle = Some(Rc::new(f));
        self
    }

    pub fn slot_left(mut self, element: Option<impl IntoElement>) -> Self {
        self.slot_left = element.map(|e| e.into_any_element());
        self
    }

    pub fn slot_right(mut self, element: Option<impl IntoElement>) -> Self {
        self.slot_right = element.map(|e| e.into_any_element());
        self
    }

    pub fn slot_bottom(mut self, element: Option<impl IntoElement>) -> Self {
        self.slot_bottom = element.map(|e| e.into_any_element());
        self
    }

    pub fn status_slot(mut self, element: Option<impl IntoElement>) -> Self {
        self.status_slot = element.map(|e| e.into_any_element());
        self
    }

    pub fn default_sizes(
        mut self,
        left: gpui::Pixels,
        right: gpui::Pixels,
        bottom: gpui::Pixels,
    ) -> Self {
        self.left_width = left;
        self.right_width = right;
        self.bottom_height = bottom;
        self
    }
}

impl Default for TabWindowShell {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for TabWindowShell {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for TabWindowShell {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let viewport = window.viewport_size();
        let show_chrome = self.show_chrome;
        let reserved = reserved_title_width(
            show_chrome,
            self.icon.is_some(),
            self.menu_slot.is_some(),
            self.title.as_ref(),
            self.title_ext_slot.is_some(),
        );
        let tabs_area = (viewport.width - reserved).max(px(160.));
        let tab_overflow = tabs_overflow(&self.tabs, tabs_area);

        let on_chrome_toggle = self.on_chrome_toggle.clone();
        let chevron = if show_chrome {
            IconName::ChevronLeft
        } else {
            IconName::ChevronRight
        };

        let chrome_toggle = self.icon.map(|app_icon| {
            Button::new("tab-window-chrome-toggle")
                .text()
                .h(TITLE_BAR_HEIGHT)
                .w(TITLE_BAR_HEIGHT)
                .flex_shrink_0()
                .rounded(ButtonRounded::None)
                .on_click(move |_, window, cx| {
                    if let Some(f) = &on_chrome_toggle {
                        f(window, cx);
                    }
                })
                .child(
                    h_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .gap_0p5()
                        .child(Icon::new(app_icon).small())
                        .child(Icon::new(chevron).small()),
                )
                .into_any_element()
        });

        let mut tab_bar = TabBar::new("tab-window-tabs")
            .menu(tab_overflow)
            .selected_index(self.selected_tab);

        // 菜单与标题随 show_chrome 展开/收起；切换按钮独立贴左，不在 prefix 内
        if show_chrome {
            let mut prefix_parts: SmallVec<[AnyElement; 2]> = SmallVec::new();
            if let Some(menu) = self.menu_slot {
                prefix_parts.push(
                    div()
                        .h_full()
                        .flex_shrink_0()
                        .child(menu)
                        .into_any_element(),
                );
            }
            if let Some(title) = self.title {
                prefix_parts.push(
                    div()
                        .px_2()
                        .flex_shrink_0()
                        .child(title)
                        .into_any_element(),
                );
            }
            if !prefix_parts.is_empty() {
                tab_bar = tab_bar.prefix(
                    h_flex()
                        .h_full()
                        .items_center()
                        .flex_shrink_0()
                        .gap_1()
                        .children(prefix_parts),
                );
            }
        }

        for tab in &self.tabs {
            let mut t = Tab::new().label(tab.label.clone());
            if let Some(icon) = tab.icon.clone() {
                t = t.icon(icon);
            }
            tab_bar = tab_bar.child(t);
        }

        if let Some(suffix) = self.title_ext_slot {
            tab_bar = tab_bar.suffix(suffix);
        }

        if let Some(on_click) = self.on_tab_click {
            tab_bar = tab_bar.on_click(move |ix, window, cx| on_click(*ix, window, cx));
        }

        let mut title_row = h_flex().h_full().w_full().min_w_0().items_center();
        if let Some(toggle) = chrome_toggle {
            title_row = title_row.child(toggle);
        }
        title_row = title_row.child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(tab_bar),
        );

        let mut title_bar = TitleBar::new().border_b_0();
        // 非 macOS：取消 TitleBar 默认左内边距，使切换按钮与窗口左上角贴合
        #[cfg(not(target_os = "macos"))]
        {
            title_bar = title_bar.pl(px(0.));
        }
        let title_bar = title_bar.child(title_row);

        let body = resizable_panel()
            .flex_1()
            .child(div().flex_1().min_h_0().size_full().children(self.children));

        let center_col = {
            let mut col = v_resizable("tab-window-center-col").child(body);
            if let Some(bottom) = self.slot_bottom {
                col = col.child(
                    resizable_panel()
                        .size(self.bottom_height)
                        .flex_none()
                        .size_range(px(80.)..px(500.))
                        .child(bottom),
                );
            }
            col
        };

        let mut main_row = h_resizable("tab-window-main-row");
        if let Some(left) = self.slot_left {
            main_row = main_row.child(
                resizable_panel()
                    .size(self.left_width)
                    .flex_none()
                    .size_range(px(48.)..px(600.))
                    .child(left),
            );
        }
        main_row = main_row.child(center_col);
        if let Some(right) = self.slot_right {
            main_row = main_row.child(
                resizable_panel()
                    .size(self.right_width)
                    .flex_none()
                    .size_range(px(160.)..px(800.))
                    .child(right),
            );
        }

        v_flex()
            .size_full()
            .child(title_bar)
            .child(div().flex_1().min_h_0().child(main_row))
            .when_some(self.status_slot, |this, slot| this.child(slot))
    }
}

//! TabWindowShell —— TabBar 标题栏 + 可调整插槽的高级窗口壳

use std::rc::Rc;

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div, px, prelude::FluentBuilder as _,
};
use gpui_component::{
    Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    tab::{Tab, TabBar},
    h_flex,
    resizable::{h_resizable, resizable_panel, v_resizable},
    v_flex,
};
use rml_core::window::WindowControlButtons;
use smallvec::SmallVec;

use super::rml_title_bar::RmlTitleBar;
use super::templates::{MenuBarTemplate, StatusBarTemplate};
use super::types::{MenuItem, StatusBarItem};

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

/// 估算 Tab 是否溢出可用宽度（约 96px / tab）
fn tabs_overflow(tab_count: usize, available_width: gpui::Pixels) -> bool {
    if tab_count == 0 {
        return false;
    }
    let estimated = px(tab_count as f32 * 96.);
    estimated > available_width
}

/// TabWindow 高级窗口壳
#[derive(IntoElement)]
pub struct TabWindowShell {
    title: Option<SharedString>,
    icon: Option<IconName>,
    show_chrome: bool,
    window_controls: WindowControlButtons,
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
            window_controls: WindowControlButtons::default(),
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

    pub fn window_controls(mut self, controls: WindowControlButtons) -> Self {
        self.window_controls = controls;
        self
    }

    pub fn menu_slot(mut self, element: impl IntoElement) -> Self {
        self.menu_slot = Some(element.into_any_element());
        self
    }

    pub fn menu(mut self, menu: Vec<MenuItem>) -> Self {
        self.menu_slot = Some(MenuBarTemplate::new(menu).into_any_element());
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

    pub fn status_slot(mut self, element: impl IntoElement) -> Self {
        self.status_slot = Some(element.into_any_element());
        self
    }

    pub fn status_bar(mut self, items: Vec<StatusBarItem>) -> Self {
        self.status_slot = Some(StatusBarTemplate::new(items).into_any_element());
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
        let tab_count = self.tabs.len();
        let viewport = window.viewport_size();
        let tabs_area = (viewport.width - px(420.)).max(px(160.));
        let tab_overflow = tabs_overflow(tab_count, tabs_area);

        let on_chrome_toggle = self.on_chrome_toggle.clone();
        let chevron = if self.show_chrome {
            IconName::ChevronLeft
        } else {
            IconName::ChevronRight
        };

        let chrome_toggle = self.icon.map(|app_icon| {
            Button::new("tab-window-chrome-toggle")
                .ghost()
                .xsmall()
                .on_click(move |_, window, cx| {
                    if let Some(f) = &on_chrome_toggle {
                        f(window, cx);
                    }
                })
                .child(
                    h_flex()
                        .items_center()
                        .gap_0p5()
                        .child(Icon::new(app_icon).small())
                        .child(Icon::new(chevron).small()),
                )
                .into_any_element()
        });

        let mut tab_bar = TabBar::new("tab-window-tabs")
            .menu(tab_overflow)
            .selected_index(self.selected_tab);

        if let Some(prefix) = self.menu_slot.filter(|_| self.show_chrome) {
            tab_bar = tab_bar.prefix(prefix);
        } else if let Some(toggle) = chrome_toggle {
            tab_bar = tab_bar.prefix(toggle);
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

        let title_bar = RmlTitleBar::new()
            .window_controls(self.window_controls)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(tab_bar),
            );

        let body = resizable_panel()
            .flex_1()
            .child(div().flex_1().min_h_0().size_full().children(self.children));

        let center_col = {
            let mut col = v_resizable("tab-window-center-col").child(body);
            if let Some(bottom) = self.slot_bottom {
                col = col.child(
                    resizable_panel()
                        .size(self.bottom_height)
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
                    .child(left),
            );
        }
        main_row = main_row.child(center_col);
        if let Some(right) = self.slot_right {
            main_row = main_row.child(
                resizable_panel()
                    .size(self.right_width)
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

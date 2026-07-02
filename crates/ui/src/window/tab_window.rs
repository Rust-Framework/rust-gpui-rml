//! TabWindowShell —— TabBar 标题栏 + 可调整插槽的高级窗口壳
//!
//! 布局（单行标题栏）：
//! `[图标切换] [菜单] [标题] [Tab…] [扩展区 suffix] [窗口操作]`
//!
//! 主体插槽：`slot_left` / `slot_right` / `slot_bottom`（可 resize，空则隐藏）、
//! `slot_footer` → `status_slot`（状态栏，空则隐藏）。

use std::rc::Rc;

use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled, Window, WindowControlArea, div, px,
    prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable as _,
    button::{Button, ButtonRounded, ButtonVariants as _},
    h_flex,
    resizable::{h_resizable, resizable_panel, v_resizable},
    v_flex, TITLE_BAR_HEIGHT,
};
use crate::components::tab::{Tab, TabBar};
use smallvec::SmallVec;

/// Slot 尺寸低于此阈值视为折叠：移出 resizable group，改用普通 div 渲染，
/// 从而隐藏 resize handle 且不污染 ResizableState 的 panel_ix 映射。
const SLOT_COLLAPSED_THRESHOLD: gpui::Pixels = px(60.);

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

/// 渲染单个窗口控件按钮（最小化/最大化/关闭）。
fn control_button(
    id: &'static str,
    icon: IconName,
    area: WindowControlArea,
    cx: &App,
) -> AnyElement {
    let is_close = matches!(area, WindowControlArea::Close);
    let hover_fg = if is_close {
        cx.theme().danger_foreground
    } else {
        cx.theme().secondary_foreground
    };
    let hover_bg = if is_close {
        cx.theme().danger
    } else {
        cx.theme().secondary_hover
    };
    let active_bg = if is_close {
        cx.theme().danger_active
    } else {
        cx.theme().secondary_active
    };

    div()
        .id(id)
        .flex()
        .w(TITLE_BAR_HEIGHT)
        .h_full()
        .flex_shrink_0()
        .justify_center()
        .content_center()
        .items_center()
        .text_color(cx.theme().foreground)
        .hover(|style| style.bg(hover_bg).text_color(hover_fg))
        .active(|style| style.bg(active_bg).text_color(hover_fg))
        .when(cfg!(target_os = "windows"), |this| {
            this.window_control_area(area)
        })
        .when(cfg!(target_os = "linux"), |this| {
            this.on_mouse_down(MouseButton::Left, |_, window, _| {
                window.prevent_default();
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                match area {
                    WindowControlArea::Min => window.minimize_window(),
                    WindowControlArea::Max => window.zoom_window(),
                    WindowControlArea::Close => window.remove_window(),
                    _ => {}
                }
            })
        })
        .child(Icon::new(icon).small())
        .into_any_element()
}

/// 渲染窗口控件组（最小化/最大化/关闭），macOS 和 wasm 返回空 div。
fn render_window_controls(window: &Window, cx: &App) -> AnyElement {
    if cfg!(target_os = "macos") || cfg!(target_family = "wasm") {
        return div().id("window-controls").into_any_element();
    }

    h_flex()
        .id("window-controls")
        .items_center()
        .flex_shrink_0()
        .h_full()
        .child(control_button(
            "minimize",
            IconName::WindowMinimize,
            WindowControlArea::Min,
            cx,
        ))
        .child(if window.is_maximized() {
            control_button(
                "restore",
                IconName::WindowRestore,
                WindowControlArea::Max,
                cx,
            )
        } else {
            control_button(
                "maximize",
                IconName::WindowMaximize,
                WindowControlArea::Max,
                cx,
            )
        })
        .child(control_button(
            "close",
            IconName::WindowClose,
            WindowControlArea::Close,
            cx,
        ))
        .into_any_element()
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

    /// 设置左侧插槽当前宽度。RML 通过 `left_size={field}` 绑定，
    /// 配合 Host 的 `cx.observe` 可实现 ActivityBar 收起时插槽自适应收回。
    pub fn left_size(mut self, size: gpui::Pixels) -> Self {
        self.left_width = size;
        self
    }

    /// 设置右侧插槽当前宽度。
    pub fn right_size(mut self, size: gpui::Pixels) -> Self {
        self.right_width = size;
        self
    }

    /// 设置底部插槽当前高度。
    pub fn bottom_size(mut self, size: gpui::Pixels) -> Self {
        self.bottom_height = size;
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let show_chrome = self.show_chrome;

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
            .menu(true)
            .flat()
            .selected_index(self.selected_tab)
            .w_full()
            .min_w_0();

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

        let mut title_row = h_flex()
            .id("tab-window-title-drag")
            .h_full()
            .flex_1()
            .min_w_0()
            .items_center()
            .when(!cfg!(target_family = "wasm"), |this| {
                this.window_control_area(WindowControlArea::Drag)
            });
        if let Some(toggle) = chrome_toggle {
            title_row = title_row.child(toggle);
        }
        title_row = title_row.child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_hidden()
                .child(tab_bar),
        );

        // 自定义 title bar：不使用 gpui-component 的 TitleBar。
        // TitleBar 内部 #bar 有 flex_shrink_0 且无 min_w_0，把 TabBar 放进去后
        // tabs 固有宽度成为 #bar 的 min-content，#bar 不收缩，窗口控件被挤出可视范围。
        // 自定义布局让 TabBar wrapper（flex_1 + min_w_0 + overflow_hidden）能自由收缩，
        // 窗口控件（flex_shrink_0）始终固定在右侧。
        //
        // window_control_area(Drag) 只设在 title_row 上，与窗口控件是兄弟关系，
        // 避免 Drag 区域覆盖按钮导致点击无效。
        let title_bar = h_flex()
            .id("tab-window-title-bar")
            .h(TITLE_BAR_HEIGHT)
            .w_full()
            .flex_shrink_0()
            .items_center()
            .bg(cx.theme().tokens.title_bar)
            .when(cfg!(target_os = "macos"), |this| this.pl(px(80.)))
            .child(title_row)
            .child(render_window_controls(window, cx));

        let body = resizable_panel()
            .flex_1()
            .child(div().flex_1().min_h_0().size_full().children(self.children));

        let bottom_collapsed = self.bottom_height <= SLOT_COLLAPSED_THRESHOLD;

        // center_col：v_resizable 始终包含 body；bottom 展开时进 v_resizable，
        // 折叠时移出 v_resizable 放到下方独立 div（无 resize handle）。
        let mut collapsed_bottom: Option<AnyElement> = None;
        let center_col = {
            let mut col = v_resizable("tab-window-center-col").child(body);
            if let Some(bottom) = self.slot_bottom {
                let panel = div()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .py(px(8.))
                    .h_full()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .bg(cx.theme().muted)
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(bottom);

                if bottom_collapsed {
                    collapsed_bottom = Some(
                        div()
                            .h(self.bottom_height)
                            .flex_none()
                            .child(panel)
                            .into_any_element(),
                    );
                } else {
                    col = col.child(
                        resizable_panel()
                            .size(self.bottom_height)
                            .flex_none()
                            .size_range(px(80.)..px(500.))
                            .child(panel),
                    );
                }
            }
            col
        };

        // 把折叠的 bottom 包到 center_col 外层 v_flex（仍在 h_resizable 内的 center panel 中）
        let center_col = if let Some(d) = collapsed_bottom {
            v_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(center_col)
                .child(d)
                .into_any_element()
        } else {
            center_col.into_any_element()
        };

        // main_row：折叠的 left/right 移出 h_resizable，避免 resize handle 残留
        // 且不污染 ResizableState 的 panel_ix 映射。
        let left_collapsed = self.left_width <= SLOT_COLLAPSED_THRESHOLD;
        let right_collapsed = self.right_width <= SLOT_COLLAPSED_THRESHOLD;

        let mut row = h_flex().w_full().h_full().min_h_0();
        let mut main_h = h_resizable("tab-window-main-row");

        match self.slot_left {
            Some(left) if left_collapsed => {
                row = row.child(
                    div()
                        .w(self.left_width)
                        .flex_none()
                        .h_full()
                        .child(left),
                );
            }
            Some(left) => {
                main_h = main_h.child(
                    resizable_panel()
                        .size(self.left_width)
                        .flex_none()
                        .size_range(px(48.)..px(600.))
                        .child(left),
                );
            }
            None => {}
        }

        main_h = main_h.child(center_col);

        let mut collapsed_right: Option<AnyElement> = None;
        match self.slot_right {
            Some(right) if right_collapsed => {
                collapsed_right = Some(right);
            }
            Some(right) => {
                main_h = main_h.child(
                    resizable_panel()
                        .size(self.right_width)
                        .flex_none()
                        .size_range(px(160.)..px(800.))
                        .child(right),
                );
            }
            None => {}
        }
        row = row.child(main_h);

        if let Some(right) = collapsed_right {
            row = row.child(
                div()
                    .w(self.right_width)
                    .flex_none()
                    .h_full()
                    .child(right),
            );
        }

        v_flex()
            .size_full()
            .child(title_bar)
            .child(div().flex_1().min_h_0().child(row))
            .when_some(self.status_slot, |this, slot| this.child(slot))
    }
}

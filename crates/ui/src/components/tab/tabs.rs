use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    Anchor, Animation, AnimationExt as _, AnyElement, App, Background, Bounds, Context, Corners,
    Div, Edges, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, ScrollHandle, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use smallvec::SmallVec;

use super::{Tab, TabItem, TabVariant};
use gpui_component::animation::{Lerp, ease_in_out_cubic};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_component::{
    ActiveTheme, ElementExt, Icon, IconName, Selectable, Sizable, Size, StyledExt, h_flex, v_flex,
};
use rust_rml_core::i18n::t_or_default;

type TabsClickHandler = Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>;
type TabsCloseAllHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

struct TabIndicatorBounds {
    container: Bounds<Pixels>,
    tabs: Vec<Bounds<Pixels>>,
}

impl TabIndicatorBounds {
    fn new(num_tabs: usize) -> Self {
        Self {
            container: Bounds::default(),
            tabs: vec![Bounds::default(); num_tabs],
        }
    }

    fn resize(&mut self, num_tabs: usize) {
        self.tabs.resize(num_tabs, Bounds::default());
    }
}

/// A Tabs element that contains multiple [`Tab`] items.
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    scroll_handle: Option<ScrollHandle>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: SmallVec<[TabItem; 2]>,
    last_empty_space: AnyElement,
    selected_index: Option<usize>,
    variant: TabVariant,
    size: Size,
    menu: bool,
    bordered: bool,
    on_click: Option<TabsClickHandler>,
    /// 选项卡关闭按钮触发时调用，参数为被关闭选项卡的索引。
    on_close: Option<TabsClickHandler>,
    /// 关闭全部 tabs 触发时调用（无索引参数）。
    on_close_all: Option<TabsCloseAllHandler>,
    /// 关闭其他 tabs 触发时调用，参数为保留 tab 的索引。
    on_close_others: Option<TabsClickHandler>,
    /// 双击 tab 触发 promote 时调用，参数为被双击 tab 的索引。
    on_promote: Option<TabsClickHandler>,
}

impl Tabs {
    /// Create a new Tabs.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            base: div().id(id),
            style: StyleRefinement::default(),
            children: SmallVec::new(),
            scroll_handle: None,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            last_empty_space: div().into_any_element(),
            selected_index: None,
            on_click: None,
            on_close: None,
            on_close_all: None,
            on_close_others: None,
            on_promote: None,
            menu: false,
            bordered: false,
        }
    }

    /// Set the Tab variant, all children will inherit the variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the Tab variant to Pill, all children will inherit the variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Set the Tab variant to Flat: borderless tabs with background-only selection.
    pub fn flat(mut self) -> Self {
        self.variant = TabVariant::Flat;
        self
    }

    /// Set the Tab variant to Outline, all children will inherit the variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    /// Set the Tab variant to Segmented, all children will inherit the variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Set the Tab variant to Underline, all children will inherit the variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set whether to show the menu button when tabs overflow, default is false.
    pub fn menu(mut self, menu: bool) -> Self {
        self.menu = menu;
        self
    }

    /// Set whether to draw a 1px border around the Tabs, default is false.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Track the scroll of the Tabs.
    pub fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }

    /// Set the prefix element of the Tabs
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the suffix element of the Tabs
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Add children of the Tabs, all children will inherit the variant.
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<TabItem>>) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    /// Add child of the Tabs, tab will inherit the variant.
    pub fn child(mut self, child: impl Into<TabItem>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Set the selected index of the Tabs.
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Set the last empty space element of the Tabs.
    pub fn last_empty_space(mut self, last_empty_space: impl IntoElement) -> Self {
        self.last_empty_space = last_empty_space.into_any_element();
        self
    }

    /// Set the on_click callback of the Tabs, the first parameter is the index of the clicked tab.
    ///
    /// When this is set, the children's on_click will be ignored.
    pub fn on_click<F>(mut self, on_click: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Set the on_close callback of the Tabs, fired when a tab's close button
    /// is clicked. The parameter is the index of the closed tab.
    ///
    /// The close button only renders on tabs whose `closable` flag is true.
    pub fn on_close<F>(mut self, on_close: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_close = Some(Rc::new(on_close));
        self
    }

    /// Set the on_close_all callback, fired when "Close All" context menu item
    /// is clicked. No index parameter — closes all tabs.
    pub fn on_close_all<F>(mut self, on_close_all: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_close_all = Some(Rc::new(on_close_all));
        self
    }

    /// Set the on_close_others callback, fired when "Close Others" context menu
    /// item is clicked. Parameter is the index of the tab to keep.
    pub fn on_close_others<F>(mut self, on_close_others: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_close_others = Some(Rc::new(on_close_others));
        self
    }

    /// Set the on_promote callback, fired when a tab is double-clicked
    /// (VSCode preview tab promote). Parameter is the index of the promoted tab.
    pub fn on_promote<F>(mut self, on_promote: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_promote = Some(Rc::new(on_promote));
        self
    }

    /// Render the sliding indicator element for animated tab switching.
    ///
    /// Returns the indicator element together with the current animation
    /// `epoch`, which increments on every tab switch. Tabs key their own
    /// transitions (e.g. text color fade) on this epoch so they restart in sync
    /// with the indicator slide.
    fn render_indicator(
        &self,
        bounds_rc: &Option<Rc<RefCell<TabIndicatorBounds>>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<(AnyElement, u64)> {
        let has_indicator = matches!(
            self.variant,
            TabVariant::Segmented | TabVariant::Pill | TabVariant::Underline
        );
        let num_tabs = self.children.len();
        let selected_ix = self.selected_index.unwrap_or(usize::MAX);

        if !(has_indicator && num_tabs > 0 && selected_ix < num_tabs) {
            return None;
        }

        let prev_key = format!("{}-tab-prev", self.id);
        let anim_key = format!("{}-tab-anim", self.id);
        let init_key = format!("{}-tab-init", self.id);

        let prev_selected = window.use_keyed_state(prev_key, cx, |_, _| selected_ix);
        // (from_left, from_width, to_left, to_width, epoch)
        let anim_params =
            window.use_keyed_state(anim_key, cx, |_, _| (px(0.), px(0.), px(0.), px(0.), 0u64));
        let initialized = window.use_keyed_state(init_key, cx, |_, _| false);

        // First frame: trigger re-render to capture bounds via on_prepaint
        if !*initialized.read(cx) {
            initialized.update(cx, |v, _| *v = true);
        }

        self.update_anim_params(selected_ix, bounds_rc, &prev_selected, &anim_params, cx);

        let (from_left, from_width, to_left, to_width, epoch) = *anim_params.read(cx);
        if to_width <= px(0.) {
            return None;
        }

        let variant = self.variant;
        let size = self.size;
        let inner_height = variant.inner_height(size);
        let inner_radius = variant.inner_radius(size, cx);

        let indicator = div()
            .absolute()
            .top_0()
            .bottom_0()
            .map(|el| match variant {
                TabVariant::Segmented => el.flex().items_center().child(
                    div()
                        .w_full()
                        .h(inner_height)
                        .bg(cx.theme().background)
                        .rounded(inner_radius)
                        .shadow_xs(),
                ),
                TabVariant::Pill => el.flex().items_center().child(
                    div()
                        .size_full()
                        .bg(cx.theme().primary)
                        .rounded(px(99.)),
                ),
                TabVariant::Underline => el.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(2.))
                        .bg(cx.theme().primary),
                ),
                _ => el,
            })
            .with_animation(
                ElementId::NamedInteger("tab-ind".into(), epoch),
                Animation::new(Duration::from_millis(200)).with_easing(ease_in_out_cubic),
                move |el, delta| {
                    let left = Lerp::lerp(&from_left, &to_left, delta);
                    let width = Lerp::lerp(&from_width, &to_width, delta);
                    el.left(left).w(width)
                },
            );

        Some((indicator.into_any_element(), epoch))
    }

    /// Update animation parameters based on current and previous selection.
    fn update_anim_params(
        &self,
        selected_ix: usize,
        bounds_rc: &Option<Rc<RefCell<TabIndicatorBounds>>>,
        prev_selected: &gpui::Entity<usize>,
        anim_params: &gpui::Entity<(Pixels, Pixels, Pixels, Pixels, u64)>,
        cx: &mut App,
    ) {
        let rc = match bounds_rc {
            Some(rc) => rc,
            None => return,
        };

        let prev_ix = *prev_selected.read(cx);
        let bounds = rc.borrow();
        let container = bounds.container;

        if container.size.width == px(0.) {
            if prev_ix != selected_ix {
                prev_selected.update(cx, |v, _| *v = selected_ix);
            }
            return;
        }

        if prev_ix != selected_ix {
            let from_b = bounds.tabs.get(prev_ix);
            let to_b = bounds.tabs.get(selected_ix);
            match (from_b, to_b) {
                (Some(from_b), Some(to_b)) => {
                    let from_left = from_b.origin.x - container.origin.x;
                    let from_width = from_b.size.width;
                    let to_left = to_b.origin.x - container.origin.x;
                    let to_width = to_b.size.width;
                    let epoch = anim_params.read(cx).4 + 1;
                    anim_params.update(cx, |v, _| {
                        *v = (from_left, from_width, to_left, to_width, epoch)
                    });
                }
                (None, Some(to_b)) => {
                    let left = to_b.origin.x - container.origin.x;
                    let width = to_b.size.width;
                    anim_params.update(cx, |v, _| *v = (left, width, left, width, v.4));
                }
                _ => {}
            }
            drop(bounds);
            prev_selected.update(cx, |v, _| *v = selected_ix);
            return;
        }

        if let Some(to_b) = bounds.tabs.get(selected_ix) {
            let left = to_b.origin.x - container.origin.x;
            let width = to_b.size.width;
            let (_, _, to_left, to_width, epoch) = *anim_params.read(cx);

            if to_width == px(0.) {
                anim_params.update(cx, |v, _| *v = (left, width, left, width, epoch));
                return;
            }

            if left != to_left || width != to_width {
                anim_params.update(cx, |v, _| *v = (left, width, left, width, epoch));
            }
        }
    }
}

impl Styled for Tabs {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Tabs {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // WPF TabControl 模式：提取选中 TabItem 的 body 闭包并立即渲染
        let body_element: Option<AnyElement> = self
            .selected_index
            .and_then(|ix| self.children.get(ix))
            .and_then(|item| item.body.clone())
            .map(|f| f(window, cx));
        let has_body = body_element.is_some();

        let default_gap = match self.size {
            Size::Small | Size::XSmall => px(8.),
            Size::Large => px(16.),
            _ => px(12.),
        };
        let (bg, paddings, gap): (Background, _, _) = match self.variant {
            TabVariant::Tab | TabVariant::Flat => {
                let padding = Edges::all(px(0.));
                (cx.theme().tab_bar.into(), padding, px(0.))
            }
            TabVariant::Outline => {
                let padding = Edges::all(px(0.));
                let bg = if has_body {
                    cx.theme().tab_bar.into()
                } else {
                    cx.theme().transparent.into()
                };
                (bg, padding, default_gap)
            }
            TabVariant::Pill => {
                let padding = Edges::all(px(0.));
                let bg = if has_body {
                    cx.theme().tab_bar.into()
                } else {
                    cx.theme().transparent.into()
                };
                (bg, padding, px(4.))
            }
            TabVariant::Segmented => {
                let padding_x = match self.size {
                    Size::XSmall => px(2.),
                    Size::Small => px(3.),
                    _ => px(4.),
                };
                let padding = Edges {
                    left: padding_x,
                    right: padding_x,
                    ..Default::default()
                };

                (cx.theme().tab_bar_segmented.into(), padding, px(2.))
            }
            TabVariant::Underline => {
                // This gap is same as the tab inner_paddings
                let gap = match self.size {
                    Size::XSmall => px(10.),
                    Size::Small => px(12.),
                    Size::Large => px(20.),
                    _ => px(16.),
                };

                (cx.theme().transparent.into(), Edges::all(px(0.)), gap)
            }
        };

        let has_indicator = matches!(
            self.variant,
            TabVariant::Segmented | TabVariant::Pill | TabVariant::Underline
        );
        let num_tabs = self.children.len();
        // 可关闭 tab 总数，用于控制 "Close All" / "Close Others" 菜单项的 disabled 状态。
        // closable=false 的 tab 不参与关闭操作（与关闭按钮可见性一致）。
        let closable_count = self.children.iter().filter(|c| c.closable).count();

        // Bounds tracking for tab indicator animation.
        // Uses Rc<RefCell> to avoid triggering re-renders from prepaint writes.
        let bounds_rc = if has_indicator && num_tabs > 0 {
            let rc: Rc<RefCell<TabIndicatorBounds>> = window
                .use_keyed_state(format!("{}-tab-bounds", self.id), cx, |_, _| {
                    Rc::new(RefCell::new(TabIndicatorBounds::new(num_tabs)))
                })
                .read(cx)
                .clone();
            rc.borrow_mut().resize(num_tabs);
            Some(rc)
        } else {
            None
        };

        let indicator = self.render_indicator(&bounds_rc, window, cx);
        let indicator_epoch = indicator.as_ref().map(|(_, epoch)| *epoch).unwrap_or(0);
        let indicator_element = indicator.map(|(el, _)| el);
        let indicator_ready = indicator_element.is_some();

        // Overflow auto-detection: measure content vs viewport width via on_prepaint
        // to auto-show the menu button. Rc<RefCell> holds measurements without
        // triggering re-renders; Entity<bool> holds the overflow flag and triggers
        // re-render on change. The init flag forces a second render to pick up
        // first-frame prepaint bounds (same pattern as the indicator code above).
        let enable_overflow = self.menu;
        let content_width_rc: Option<Rc<RefCell<Pixels>>> = if enable_overflow {
            Some(
                window
                    .use_keyed_state(format!("{}-content-w", self.id), cx, |_, _| {
                        Rc::new(RefCell::new(px(0.)))
                    })
                    .read(cx)
                    .clone(),
            )
        } else {
            None
        };
        let viewport_width_rc: Option<Rc<RefCell<Pixels>>> = if enable_overflow {
            Some(
                window
                    .use_keyed_state(format!("{}-viewport-w", self.id), cx, |_, _| {
                        Rc::new(RefCell::new(px(0.)))
                    })
                    .read(cx)
                    .clone(),
            )
        } else {
            None
        };
        let overflow_state: Option<Entity<bool>> = if enable_overflow {
            let state =
                window.use_keyed_state(format!("{}-overflow", self.id), cx, |_, _| false);
            let init =
                window.use_keyed_state(format!("{}-overflow-init", self.id), cx, |_, _| false);
            if !*init.read(cx) {
                init.update(cx, |v, _| *v = true);
            }
            // Re-evaluate overflow using the previous frame's measurements.
            let cw = *content_width_rc.as_ref().unwrap().borrow();
            let vw = *viewport_width_rc.as_ref().unwrap().borrow();
            let new_overflow = cw > vw + px(0.5);
            if new_overflow != *state.read(cx) {
                state.update(cx, |v, _| *v = new_overflow);
            }
            Some(state)
        } else {
            None
        };

        let show_menu =
            self.menu && overflow_state.as_ref().map(|s| *s.read(cx)).unwrap_or(false);
        // Raw overflow flag (not gated by `self.menu`): when true, tabs switch
        // from fixed-width scroll mode to browser-like compression (flex_1 +
        // min_w_0 + label ellipsis).
        let is_overflow = overflow_state.as_ref().map(|s| *s.read(cx)).unwrap_or(false);
        let has_suffix_or_menu = self.suffix.is_some() || show_menu;
        let mut item_metas: Vec<(Option<SharedString>, Option<Icon>, bool)> = Vec::new();
        let selected_index = self.selected_index;
        let on_click = self.on_click.clone();

        // 测量层：独立的轻量级 Tab 列表，始终 flex_shrink_0 以测得自然内容宽度，
        // 不参与 is_overflow 压缩，避免测量反馈循环（is_overflow 翻转导致测量元素
        // 在 flex_shrink_0/flex_1 之间切换，进而反复触发 is_overflow 翻转）。
        let variant = self.variant;
        let size = self.size;
        let measurement_tabs: Vec<Tab> = if enable_overflow {
            self.children
                .iter()
                .enumerate()
                .map(|(ix, item)| {
                    let mut tab = Tab::new()
                        .ix(ix)
                        .tab_bar_prefix(item.tab_bar_prefix.unwrap_or(true))
                        .disabled(item.disabled)
                        .closable(item.closable)
                        .with_variant(variant)
                        .with_size(size)
                        .measurement();
                    if let Some(label) = &item.title_label {
                        tab = tab.label(label.clone());
                    }
                    if let Some(icon) = &item.title_icon {
                        tab = tab.icon(icon.clone());
                    }
                    tab
                })
                .collect()
        } else {
            Vec::new()
        };

        let header = self.base
            .group("tab-bar")
            .relative()
            .flex()
            .items_center()
            .bg(bg)
            .text_color(cx.theme().tab_foreground)
            .when(
                self.variant == TabVariant::Underline
                    || (has_body
                        && matches!(self.variant, TabVariant::Outline | TabVariant::Pill)),
                |this| this.border_b_1().border_color(cx.theme().border),
            )
            .when_else(
                self.variant == TabVariant::Segmented && has_body,
                |this| {
                    this.corner_radii(Corners {
                        top_left: self.variant.tab_bar_radius(self.size, cx),
                        top_right: self.variant.tab_bar_radius(self.size, cx),
                        bottom_left: px(0.),
                        bottom_right: px(0.),
                    })
                },
                |this| this.rounded(self.variant.tab_bar_radius(self.size, cx)),
            )
            .paddings(paddings)
            .when_some(content_width_rc.clone(), |this, cw_rc| {
                // 独立测量层：absolute 出流 + opacity:0，
                // 始终 flex_shrink_0 测量自然内容宽度，不受 is_overflow 压缩影响。
                // 放在 header（relative）内而非 tabs-inner 内，避免被 overflow_x_hidden 裁剪。
                // DOM 顺序在显示层之前，确保显示层在上层接收鼠标事件。
                this.child(
                    h_flex()
                        .id("tab-bar-measure")
                        .absolute()
                        .top_0()
                        .left_0()
                        .opacity(0.)
                        .flex_shrink_0()
                        .gap(gap)
                        .on_prepaint(move |bounds, _, _| {
                            *cw_rc.borrow_mut() = bounds.size.width;
                        })
                        .children(measurement_tabs),
                )
            })
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                h_flex()
                    .id("tabs")
                    .flex_1()
                    .overflow_x_hidden()
                    .when_some(viewport_width_rc.clone(), |this, vw_rc| {
                        this.on_prepaint(move |bounds, _, _| {
                            *vw_rc.borrow_mut() = bounds.size.width;
                        })
                    })
                    .child(
                        h_flex()
                            .id("tabs-inner")
                            .relative()
                            .when(is_overflow, |this| this.overflow_x_hidden())
                            .when(!is_overflow, |this| this.overflow_x_scroll())
                            .when_some(self.scroll_handle, |this, scroll_handle| {
                                this.track_scroll(&scroll_handle)
                            })
                            .when_some(bounds_rc.clone(), |this, rc| {
                                this.on_prepaint(move |bounds, _, _| {
                                    rc.borrow_mut().container = bounds;
                                })
                            })
                            .when_some(indicator_element, |this, ind| this.child(ind))
                            .child(
                                h_flex()
                                    .gap(gap)
                                    .when_else(is_overflow, |this| this.flex_1().min_w_0(), |this| this.flex_shrink_0())
                                    .children(self.children.into_iter().enumerate().map(|(ix, item)| {
                                        item_metas.push((
                                            item.title_label.clone(),
                                            item.title_icon.clone(),
                                            item.disabled,
                                        ));
                                        // 当前 tab 的 closable 状态：控制 "Close" 菜单项是否显示。
                                        let tab_closable = item.closable;
                                        // 其他可关闭 tab 数量：控制 "Close Others" disabled 状态。
                                        let other_closable_count =
                                            closable_count - if tab_closable { 1 } else { 0 };
                                        let tab_bar_prefix = item.tab_bar_prefix.unwrap_or(true);
                                        let mut tab = item
                                            .ix(ix)
                                            .tab_bar_prefix(tab_bar_prefix)
                                            .into_header_tab()
                                            .with_variant(self.variant)
                                            .with_size(self.size)
                                            .compress(is_overflow);
                                        tab.indicator_active = has_indicator;
                                        tab.indicator_ready = indicator_ready;
                                        tab.indicator_epoch = indicator_epoch;
                                        let tab = tab
                                            .when_some(self.selected_index, |this, selected_ix| {
                                                this.selected(selected_ix == ix)
                                            })
                                            .when_some(self.on_click.clone(), move |this, on_click| {
                                                this.on_click(move |_, window, cx| on_click(&ix, window, cx))
                                            })
                                            .when_some(self.on_close.clone(), move |this, on_close| {
                                                this.on_close(move |_, window, cx| on_close(&ix, window, cx))
                                            })
                                            .when_some(self.on_promote.clone(), move |this, on_promote| {
                                                this.on_promote(move |window, cx| on_promote(&ix, window, cx))
                                            });

                                        // 框架内置右键菜单：Close / Close Others / Close All。
                                        // 仅在业务层提供至少一个回调时挂载，菜单项文本走 i18n。
                                        // closable=false 的 tab 不显示 "Close" 项（与关闭按钮可见性一致），
                                        // 但仍显示 "Close Others" / "Close All"（可关闭其他 tab）。
                                        let on_close_for_menu = self.on_close.clone();
                                        let on_close_all = self.on_close_all.clone();
                                        let on_close_others = self.on_close_others.clone();
                                        let has_context_menu = on_close_for_menu.is_some()
                                            || on_close_all.is_some()
                                            || on_close_others.is_some();
                                        let tab = if has_context_menu {
                                            let provider = move |mut menu: PopupMenu,
                                                                 _window: &mut Window,
                                                                 cx: &mut Context<PopupMenu>|
                                                  -> PopupMenu {
                                                // "Close" 项：仅当当前 tab 可关闭时显示
                                                if tab_closable {
                                                    if let Some(on_close) =
                                                        on_close_for_menu.clone()
                                                    {
                                                        let idx = ix;
                                                        menu = menu.item(
                                                            PopupMenuItem::new(t_or_default(
                                                                cx,
                                                                "rml.tab.close",
                                                                "Close",
                                                            ))
                                                            .on_click(move |_, w, c| {
                                                                on_close(&idx, w, c)
                                                            }),
                                                        );
                                                    }
                                                }
                                                // "Close Others" 项：其他可关闭 tab 数为 0 时禁用
                                                if let Some(on_close_others) =
                                                    on_close_others.clone()
                                                {
                                                    let idx = ix;
                                                    let disabled = other_closable_count == 0;
                                                    menu = menu.item(
                                                        PopupMenuItem::new(t_or_default(
                                                            cx,
                                                            "rml.tab.close_others",
                                                            "Close Others",
                                                        ))
                                                        .disabled(disabled)
                                                        .on_click(move |_, w, c| {
                                                            on_close_others(&idx, w, c)
                                                        }),
                                                    );
                                                }
                                                // "Close All" 项：无可关闭 tab 时禁用
                                                if let Some(on_close_all) = on_close_all.clone() {
                                                    let disabled = closable_count == 0;
                                                    menu = menu.item(
                                                        PopupMenuItem::new(t_or_default(
                                                            cx,
                                                            "rml.tab.close_all",
                                                            "Close All",
                                                        ))
                                                        .disabled(disabled)
                                                        .on_click(move |_, w, c| {
                                                            on_close_all(w, c)
                                                        }),
                                                    );
                                                }
                                                menu
                                            };
                                            tab.context_menu_provider(provider)
                                        } else {
                                            tab
                                        };

                                        if let Some(ref rc) = bounds_rc {
                                            let rc = rc.clone();
                                            div()
                                                .on_prepaint(move |bounds, _, _| {
                                                    if let Some(slot) = rc.borrow_mut().tabs.get_mut(ix) {
                                                        *slot = bounds;
                                                    }
                                                })
                                                .child(tab)
                                                .into_any_element()
                                        } else {
                                            tab.into_any_element()
                                        }
                                    }))
                                    .when(has_suffix_or_menu, |this| this.child(div().w(gap).child(self.last_empty_space))),
                            ),
                    ),
            )
            .when(show_menu, |this| {
                this.child(
                    Button::new("more")
                        .xsmall()
                        .ghost()
                        .icon(IconName::ChevronDown)
                        .dropdown_menu(move |mut this, _, _| {
                            this = this.scrollable(true);
                            for (ix, (label, icon, disabled)) in item_metas.iter().enumerate() {
                                let base = if let Some(label) = label.clone() {
                                    PopupMenuItem::new(label)
                                } else if let Some(icon) = icon.clone() {
                                    PopupMenuItem::element(move |_, _| icon.clone())
                                } else {
                                    PopupMenuItem::new("Unnamed")
                                };
                                this = this.item(
                                    base.checked(selected_index == Some(ix))
                                        .disabled(*disabled)
                                        .when_some(on_click.clone(), |this, on_click| {
                                            this.on_click(move |_, window, cx| {
                                                on_click(&ix, window, cx)
                                            })
                                        }),
                                );
                            }

                            this
                        })
                        .anchor(Anchor::TopRight),
                )
            })
            .when_some(self.suffix, |this, suffix| this.child(suffix));

        // WPF TabControl 模式：当存在 body 时，垂直堆叠 header + body
        // bordered 包裹 header + body 整体（而非仅 header）
        // self.style（含用户 inline style + 归一化属性）应用到最外层元素，
        // 确保用户 style 作用于整个组件而非仅 tab strip。
        match body_element {
            Some(body) => v_flex()
                .size_full()
                .items_stretch()
                .refine_style(&self.style)
                .when(self.bordered, |this| {
                    this.border_1().border_color(cx.theme().border)
                })
                .child(header)
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .bg(cx.theme().background)
                        .child(body),
                )
                .into_any_element(),
            None => header.refine_style(&self.style).into_any_element(),
        }
    }
}
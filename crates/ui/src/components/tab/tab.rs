use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{ContextMenuExt, OverflowStyle, PopupMenu, Tooltip};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, px, relative, Animation, AnimationExt as _, AnyElement, App, Background, ClickEvent,
    Context, Corners, Div, Edges, ElementId, Hsla, InteractiveElement, IntoElement,
    MouseButton, Overflow, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::animation::{ease_in_out_cubic, Lerp};
use gpui_component::{h_flex, ActiveTheme, Icon, IconName, Selectable, Sizable, Size, StyledExt};
use rust_rml_core::i18n::t_or_default;

type TabClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type TabContextMenuProvider =
    Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>;
type TabPromoteHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// 压缩模式下激活 tab 的最小宽度下限。
///
/// 确保激活 tab 标题在多 tab 溢出压缩时仍可读（约 6-8 个中文字符或图标+短文本+关闭按钮）。
/// 非激活 tab 不设下限，可完全压缩以优先保障激活项可见性。
const COMPRESS_ACTIVE_MIN_W: Pixels = px(120.);

/// Tab variants.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum TabVariant {
    #[default]
    Tab,
    Flat,
    Outline,
    Pill,
    Segmented,
    Underline,
}

impl TabVariant {
    fn height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Underline => px(26.),
                _ => px(20.),
            },
            Size::Small => match self {
                TabVariant::Underline => px(30.),
                _ => px(24.),
            },
            Size::Large => match self {
                TabVariant::Underline => px(44.),
                _ => px(36.),
            },
            _ => match self {
                TabVariant::Underline => px(36.),
                _ => px(32.),
            },
        }
    }

    pub(super) fn inner_height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Tab | TabVariant::Flat | TabVariant::Outline | TabVariant::Pill => {
                    px(18.)
                }
                TabVariant::Segmented => px(16.),
                TabVariant::Underline => px(20.),
            },
            Size::Small => match self {
                TabVariant::Tab | TabVariant::Flat | TabVariant::Outline | TabVariant::Pill => {
                    px(22.)
                }
                TabVariant::Segmented => px(18.),
                TabVariant::Underline => px(22.),
            },
            Size::Large => match self {
                TabVariant::Tab | TabVariant::Flat | TabVariant::Outline | TabVariant::Pill => {
                    px(36.)
                }
                TabVariant::Segmented => px(28.),
                TabVariant::Underline => px(32.),
            },
            _ => match self {
                TabVariant::Tab | TabVariant::Flat => px(30.),
                TabVariant::Outline | TabVariant::Pill => px(26.),
                TabVariant::Segmented => px(24.),
                TabVariant::Underline => px(26.),
            },
        }
    }

    /// Outer row height of a tab for layout in chrome such as [`super::Tabs`].
    pub fn tab_height(self, size: Size) -> Pixels {
        self.height(size)
    }

    /// Default px(12) to match panel px_3, See [`crate::dock::TabPanel`]
    fn inner_paddings(&self, size: Size) -> Edges<Pixels> {
        let mut padding_x = match size {
            Size::XSmall => px(8.),
            Size::Small => px(10.),
            Size::Large => px(16.),
            _ => px(12.),
        };

        if matches!(self, TabVariant::Underline) {
            padding_x = px(0.);
        }

        Edges {
            left: padding_x,
            right: padding_x,
            ..Default::default()
        }
    }

    fn inner_margins(&self, size: Size) -> Edges<Pixels> {
        match size {
            Size::XSmall => match self {
                TabVariant::Underline => Edges {
                    top: px(1.),
                    bottom: px(2.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            Size::Small => match self {
                TabVariant::Underline => Edges {
                    top: px(2.),
                    bottom: px(3.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            Size::Large => match self {
                TabVariant::Underline => Edges {
                    top: px(5.),
                    bottom: px(6.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            _ => match self {
                TabVariant::Underline => Edges {
                    top: px(3.),
                    bottom: px(4.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
        }
    }

    fn normal(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Flat => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
        }
    }

    fn hovered(&self, selected: bool, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Flat => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().secondary_hover.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().primary,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().secondary.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: if selected {
                    cx.theme().background.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
        }
    }

    fn selected(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().tab_active.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Flat => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().tab_active.into(),
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().primary,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().primary,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().primary_foreground,
                bg: cx.theme().primary.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().background.into(),
                shadow: true,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().primary,
                ..Default::default()
            },
        }
    }

    fn disabled(&self, selected: bool, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: if selected {
                    cx.theme().tab_active.into()
                } else {
                    cx.theme().transparent.into()
                },
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                ..Default::default()
            },
            TabVariant::Flat => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: if selected {
                    cx.theme().tab_active.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: if selected {
                    cx.theme().primary_foreground.opacity(0.5)
                } else {
                    cx.theme().muted_foreground
                },
                bg: if selected {
                    cx.theme().primary.opacity(0.5).into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: if selected {
                    cx.theme().background.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    pub(super) fn tab_bar_radius(&self, size: Size, cx: &App) -> Pixels {
        if *self != TabVariant::Segmented {
            return px(0.);
        }

        match size {
            Size::XSmall | Size::Small => cx.theme().radius,
            Size::Large => cx.theme().radius_lg,
            _ => cx.theme().radius_lg,
        }
    }

    fn radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Outline | TabVariant::Pill => px(99.),
            TabVariant::Segmented => match size {
                Size::XSmall | Size::Small => cx.theme().radius,
                Size::Large => cx.theme().radius_lg,
                _ => cx.theme().radius_lg,
            },
            _ => px(0.),
        }
    }

    fn corner_radii(
        &self,
        size: Size,
        selected: bool,
        disabled: bool,
        cx: &App,
    ) -> Corners<Pixels> {
        let _ = (selected, disabled);
        Corners::all(self.radius(size, cx))
    }

    pub(super) fn inner_radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Segmented => match size {
                Size::Large => self.tab_bar_radius(size, cx) - px(3.),
                _ => self.tab_bar_radius(size, cx) - px(2.),
            },
            _ => px(0.),
        }
    }
}

struct TabStyle {
    borders: Edges<Pixels>,
    border_color: Hsla,
    bg: Background,
    fg: Hsla,
    shadow: bool,
    inner_bg: Background,
}

impl Default for TabStyle {
    fn default() -> Self {
        TabStyle {
            borders: Edges::all(px(0.)),
            border_color: gpui::transparent_white(),
            bg: gpui::transparent_white().into(),
            fg: gpui::transparent_white(),
            shadow: false,
            inner_bg: gpui::transparent_white().into(),
        }
    }
}

/// A Tab element for the [`super::Tabs`] or [`super::TabBar`].
#[derive(IntoElement)]
pub struct Tab {
    pub(super) ix: usize,
    base: Div,
    pub(super) label: Option<SharedString>,
    pub(super) icon: Option<Icon>,
    prefix: Option<AnyElement>,
    pub(super) tab_bar_prefix: Option<bool>,
    suffix: Option<AnyElement>,
    pub(super) children: Vec<AnyElement>,
    variant: TabVariant,
    size: Size,
    pub(super) disabled: bool,
    pub(super) selected: bool,
    pub(super) indicator_active: bool,
    pub(super) indicator_ready: bool,
    /// Animation epoch of the [`super::Tabs`] indicator; increments on every
    /// tab switch. Used to key the selected tab's text color fade so it
    /// restarts in sync with the indicator slide.
    pub(super) indicator_epoch: u64,
    pub(super) on_click: Option<TabClickHandler>,
    /// When true, render a close button at the end of the tab. The button is
    /// only visible while the parent tab is hovered, and its click is wired to
    /// `on_close` when set.
    pub(super) closable: bool,
    pub(super) on_close: Option<TabClickHandler>,
    /// 右键菜单构造器，由 Tabs 透传。闭包接收框架传入的 `PopupMenu`，
    /// 追加标准项（Close/Close All/Close Others）+ 业务扩展项后返回。
    pub(super) context_menu_provider: Option<TabContextMenuProvider>,
    /// When true, render the label in italic (VSCode preview tab style).
    /// Only affects the label branch; icon and custom children are unchanged.
    pub(super) preview: bool,
    /// 双击 tab 时触发（VSCode preview tab promote）。由 Tabs 透传，
    /// 内部在 on_mouse_down 中检测 250ms 时间窗口内的双击。
    pub(super) on_promote: Option<TabPromoteHandler>,
    /// When true, the tab shrinks to share width with siblings (browser-like
    /// compression). Switches from `flex_shrink_0` to `flex_1 + min_w_0` and
    /// enables label ellipsis truncation. Set by Tabs when overflow detected.
    pub(super) compress: bool,
    /// Set by Tabs when the selected tab should merge with a body panel below.
    pub(super) connect_body: bool,
    /// When true, render as a measurement-only element: skip all interactions
    /// (hover, group, on_click, on_mouse_down) but keep visual layout — including
    /// the close button's width — for accurate width measurement. Used by
    /// Tabs' independent measurement layer to avoid the overflow feedback loop.
    pub(super) measurement: bool,
}

impl From<&'static str> for Tab {
    fn from(label: &'static str) -> Self {
        Self::new().label(label)
    }
}

impl From<String> for Tab {
    fn from(label: String) -> Self {
        Self::new().label(label)
    }
}

impl From<SharedString> for Tab {
    fn from(label: SharedString) -> Self {
        Self::new().label(label)
    }
}

impl From<Icon> for Tab {
    fn from(icon: Icon) -> Self {
        Self::default().icon(icon)
    }
}

impl From<IconName> for Tab {
    fn from(icon_name: IconName) -> Self {
        Self::default().icon(Icon::new(icon_name))
    }
}

impl Default for Tab {
    fn default() -> Self {
        Self {
            ix: 0,
            base: div(),
            label: None,
            icon: None,
            tab_bar_prefix: None,
            children: Vec::new(),
            disabled: false,
            selected: false,
            indicator_active: false,
            indicator_ready: true,
            indicator_epoch: 0,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            on_click: None,
            closable: false,
            on_close: None,
            context_menu_provider: None,
            preview: false,
            on_promote: None,
            compress: false,
            connect_body: false,
            measurement: false,
        }
    }
}

impl Tab {
    /// Create a new tab with a label.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set label for the tab.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set icon for the tab.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set Tab Variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Use Flat variant: borderless tabs with background-only selection.
    pub fn flat(mut self) -> Self {
        self.variant = TabVariant::Flat;
        self
    }

    /// Use Pill variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Use outline variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    /// Use Segmented variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Use Underline variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set the left side of the tab
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the right side of the tab
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set disabled state to the tab, default false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the click handler for the tab.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// When true, render a close button at the end of the tab. Visibility of
    /// the button is driven by parent-tab hover, and its click is wired to the
    /// `on_close` handler when set.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// When true, render the label in italic (VSCode preview tab style).
    /// Only affects the label branch; icon and custom children are unchanged.
    pub fn preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    /// 双击 tab 时触发 promote 回调。由 TabBar 透传，Tab 内部在
    /// `on_mouse_down` 中检测 250ms 时间窗口内的双击。
    pub(crate) fn on_promote(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_promote = Some(Rc::new(handler));
        self
    }

    /// Set the close handler invoked when the tab's close button is clicked.
    /// The handler receives the same `(ClickEvent, Window, App)` signature as
    /// `on_click`; the close button additionally calls `stop_propagation` so
    /// the click does not also trigger tab selection.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(on_close));
        self
    }

    /// Set the context menu provider for right-click menu.
    /// The provider receives a `PopupMenu` from the framework, appends items,
    /// and returns the modified menu. TabBar transparently passes standard
    /// Close/Close All/Close Others handlers via this closure.
    pub(crate) fn context_menu_provider(
        mut self,
        provider: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.context_menu_provider = Some(Rc::new(provider));
        self
    }

    /// Enable browser-like tab compression: the tab switches from fixed-width
    /// (`flex_shrink_0`) to flexible (`flex_1 + min_w_0`) and the label gets
    /// ellipsis truncation. Set by TabBar when overflow is detected.
    pub(crate) fn compress(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }

    /// When true, extend the selected tab to cover the tab-strip bottom border
    /// so it merges flush with a body panel below.
    pub(crate) fn connect_body(mut self, connect_body: bool) -> Self {
        self.connect_body = connect_body;
        self
    }

    /// Render as a measurement-only element: skip all interactions but keep
    /// visual layout for accurate width measurement. Used by TabBar's
    /// independent measurement layer.
    pub(crate) fn measurement(mut self) -> Self {
        self.measurement = true;
        self
    }

    /// Set index to the tab.
    pub(crate) fn ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self
    }

    /// Set if the tab bar has a prefix.
    pub(crate) fn tab_bar_prefix(mut self, tab_bar_prefix: bool) -> Self {
        self.tab_bar_prefix = Some(tab_bar_prefix);
        self
    }
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Selectable for Tab {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl Styled for Tab {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl Sizable for Tab {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Tab {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let m = self.measurement;
        let mut tab_style = if self.selected {
            self.variant.selected(cx)
        } else {
            self.variant.normal(cx)
        };
        let mut hover_style = self.variant.hovered(self.selected, cx);
        if self.disabled {
            tab_style = self.variant.disabled(self.selected, cx);
            hover_style = self.variant.disabled(self.selected, cx);
        }
        if self.ix == 0 && self.variant == TabVariant::Tab {
            tab_style.borders.left = px(0.);
            hover_style.borders.left = px(0.);
        }
        let corner_radii = self
            .variant
            .corner_radii(self.size, self.selected, self.disabled, cx);
        let inner_radius = self.variant.inner_radius(self.size, cx);
        let mut inner_paddings = self.variant.inner_paddings(self.size);
        // 有关闭按钮时缩小右内边距，让关闭按钮紧贴文本
        if self.closable {
            inner_paddings.right = px(4.);
        }
        let inner_margins = self.variant.inner_margins(self.size);
        let inner_height = self.variant.inner_height(self.size);
        let height = self.variant.height(self.size);

        let segmented_indicator_active =
            self.variant == TabVariant::Segmented && self.indicator_active;
        let has_inline_inner_bg =
            self.selected && segmented_indicator_active && !self.indicator_ready;
        let inline_inner_bg = tab_style.inner_bg;
        let (inner_bg, hover_inner_bg) = if segmented_indicator_active && self.indicator_ready {
            (cx.theme().transparent.into(), cx.theme().transparent.into())
        } else if has_inline_inner_bg {
            (inline_inner_bg, inline_inner_bg)
        } else {
            (tab_style.inner_bg, hover_style.inner_bg)
        };
        let inner_shadow = tab_style.shadow && !segmented_indicator_active;

        // When a sliding indicator is active, it alone represents the selected
        // state. Suppress the selected tab's own active background/border so the
        // two don't overlap (Segmented already does this for its `inner_bg`
        // above). Suppress regardless of indicator_ready to avoid first-frame
        // flash: the selected tab shows normal styling until the indicator
        // appears, then the indicator takes over. Skip disabled tabs so a
        // disabled-selected tab keeps its dimmed styling instead of the
        // full-strength indicator color.
        let suppress_active_visual = self.selected && !self.disabled && self.indicator_active;
        // Pill paints its active state via the outer `bg`.
        let outer_bg = if suppress_active_visual && self.variant == TabVariant::Pill {
            cx.theme().transparent.into()
        } else {
            tab_style.bg
        };
        // Underline paints its active state via the bottom `border_color`.
        let outer_border_color = if suppress_active_visual && self.variant == TabVariant::Underline
        {
            cx.theme().transparent
        } else {
            tab_style.border_color
        };

        // For Pill, the newly selected tab's text color (`primary_foreground`)
        // would otherwise snap to white instantly while the indicator is still
        // sliding into place. Fade it from the normal color in sync with the
        // indicator slide (keyed on the indicator epoch so it restarts on each
        // switch). `epoch == 0` is the initial layout (no slide), so we skip it.
        let animate_fg = self.selected
            && !self.disabled
            && self.variant == TabVariant::Pill
            && self.indicator_active
            && self.indicator_ready
            && self.indicator_epoch > 0;
        let fg_from = self.variant.normal(cx).fg;
        let fg_to = tab_style.fg;

        let inner_content = h_flex()
            .flex_1()
            .h(inner_height)
            .line_height(relative(1.))
            .whitespace_nowrap()
            .items_center()
            .overflow_hidden()
            .margins(inner_margins)
            .when_else(
                self.compress,
                |this| this.min_w_0(),
                |this| this.flex_shrink_0(),
            )
            .map(|this| match self.icon {
                Some(icon) => this
                    .w(inner_height * 1.25)
                    .child(icon.map(|this| match self.size {
                        Size::XSmall => this.size_2p5(),
                        Size::Small => this.size_3p5(),
                        Size::Large => this.size_4(),
                        _ => this.size_4(),
                    })),
                None => this
                    .paddings(inner_paddings)
                    .map(|this| match self.label {
                        Some(label) => {
                            if self.compress {
                                this.child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .when(self.preview, |t| t.italic())
                                        .child(label),
                                )
                            } else if self.preview {
                                this.child(div().italic().child(label))
                            } else {
                                this.child(label)
                            }
                        }
                        None => this,
                    })
                    .children(self.children),
            })
            .bg(inner_bg)
            .rounded(inner_radius)
            .when(inner_shadow, |this| this.shadow_xs())
            .hover(|this| this.bg(hover_inner_bg).rounded(inner_radius));

        let inner_element = if animate_fg {
            inner_content
                .with_animation(
                    ElementId::NamedInteger("tab-fg".into(), self.indicator_epoch),
                    Animation::new(Duration::from_millis(200)).with_easing(ease_in_out_cubic),
                    move |this, delta| this.text_color(Lerp::lerp(&fg_from, &fg_to, delta)),
                )
                .into_any_element()
        } else {
            inner_content.into_any_element()
        };

        // Per-tab group id used to drive close-button visibility via
        // `group_hover` (parent hover → child opacity 1).
        let group_name = format!("tab-{}", self.ix);

        let merge_with_body = self.selected && self.connect_body && !m;
        let body_merge_color = cx.theme().tab_active;

        let base = self
            .base
            .id(self.ix)
            .when(self.closable && !m, |this| this.group(group_name.clone()))
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .when_else(
                self.compress,
                |this| {
                    this.flex_1()
                        .when(self.selected, |this| this.min_w(COMPRESS_ACTIVE_MIN_W))
                        .when(!self.selected, |this| this.min_w_0())
                },
                |this| this.flex_shrink_0(),
            )
            .h(height)
            .when_else(
                merge_with_body,
                |this| {
                    // Extend the active tab 1px over the strip separator and pull the
                    // body up so layout height stays unchanged.
                    this.overflow(Overflow::Visible).pb(px(1.)).mb(-px(1.))
                },
                |this| this.overflow_hidden(),
            )
            .text_color(tab_style.fg)
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Large => this.text_base(),
                _ => this.text_sm(),
            })
            .bg(outer_bg)
            .border_l(tab_style.borders.left)
            .border_r(tab_style.borders.right)
            .border_t(tab_style.borders.top)
            .border_b(if merge_with_body {
                px(0.)
            } else {
                tab_style.borders.bottom
            })
            .border_color(outer_border_color)
            .corner_radii(corner_radii)
            .when(!self.selected && !self.disabled && !m, |this| {
                this.hover(|this| {
                    this.text_color(hover_style.fg)
                        .bg(hover_style.bg)
                        .border_l(hover_style.borders.left)
                        .border_r(hover_style.borders.right)
                        .border_t(hover_style.borders.top)
                        .border_b(hover_style.borders.bottom)
                        .border_color(hover_style.border_color)
                        .corner_radii(corner_radii)
                })
            })
            .when(has_inline_inner_bg, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .w_full()
                                .h(inner_height)
                                .bg(inline_inner_bg)
                                .rounded(inner_radius)
                                .when(tab_style.shadow, |this| this.shadow_xs()),
                        ),
                )
            })
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(inner_element)
            .when_some(self.suffix, |this, suffix| this.child(suffix))
            .when(merge_with_body, |this| {
                // 2px cover: 1px over the strip separator + 1px into the body panel.
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom(-px(1.))
                        .h(px(2.))
                        .bg(body_merge_color),
                )
            })
            .when(self.closable && !self.disabled, |this| {
                // 激活 tab 常显；非激活 tab 仅在父 tab hover 时显示
                let btn_size = match self.size {
                    Size::XSmall => px(14.),
                    Size::Small => px(16.),
                    Size::Large => px(20.),
                    _ => px(18.),
                };
                let close_btn = if m {
                    // 测量模式：尺寸须与正式按钮一致，确保宽度测量准确
                    div()
                        .opacity(0.)
                        .size(btn_size)
                        .flex()
                        .items_center()
                        .justify_center()
                        .mr(px(4.))
                        .child(Icon::new(IconName::Close).small())
                        .into_any_element()
                } else {
                    let on_close = self.on_close.clone();
                    let hover_bg = cx.theme().secondary_hover;
                    div()
                        .id(("tab-close", self.ix))
                        .size(btn_size)
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .mr(px(4.))
                        .cursor_pointer()
                        .when(!self.selected, |this| {
                            this.opacity(0.)
                                .group_hover(group_name.clone(), |this| this.opacity(1.))
                        })
                        .hover(move |this| this.bg(hover_bg))
                        .tooltip(move |window, cx| {
                            Tooltip::new(t_or_default(cx, "rml.tab.close", "Close"))
                                .build(window, cx)
                        })
                        .child(Icon::new(IconName::Close).small())
                        .when_some(on_close, |this, on_close| {
                            this.on_click(move |event, window, cx| {
                                cx.stop_propagation();
                                on_close(event, window, cx);
                            })
                        })
                        .into_any_element()
                };
                this.child(close_btn)
            })
            .when(!m, |this| {
                let ix = self.ix;
                let on_promote = self.on_promote.clone();
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    // Stop propagation behavior, for works on TitleBar.
                    // https://github.com/longbridge/gpui-component/issues/1836
                    cx.stop_propagation();
                    // 双击检测：250ms 时间窗口内两次点击触发 promote。
                    // 双击后清空状态，避免三击误触发。
                    let now = Instant::now();
                    let dbl_key = format!("tab-dbl-{}", ix);
                    let is_dbl =
                        crate::components::dbl_click::check_double_click(cx, &dbl_key, now);
                    if is_dbl {
                        if let Some(on_promote) = &on_promote {
                            on_promote(window, cx);
                        }
                    }
                })
            })
            .when(!m && !self.disabled, |this| {
                this.when_some(self.on_click.clone(), |this, on_click| {
                    this.on_click(move |event, window, cx| on_click(event, window, cx))
                })
            });

        // context_menu 返回 ContextMenu<Stateful<Div>>，与链上 Stateful<Div> 类型不同，
        // 故从 .when() 链中移出，在链尾条件性挂载并统一转为 AnyElement。
        if !m {
            if let Some(provider) = self.context_menu_provider.clone() {
                base.context_menu(move |menu, window, cx| provider(menu, window, cx))
                    .into_any_element()
            } else {
                base.into_any_element()
            }
        } else {
            base.into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    // is_double_click 纯函数测试已迁移至共享模块 `crate::components::dbl_click`,
    // 由该模块统一覆盖(含 boundary / beyond_window / check_and_update 等场景)。
}

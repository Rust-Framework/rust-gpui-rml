//! Card 组件 —— Ant Design Card 标准封装
//!
//! 基于 gpui::div() + gpui-component 主题构建的卡片容器，提供：
//! - `title` / `extra`：标题栏（左标题 + 右侧附加区）
//! - `cover`：顶部封面图（位于标题之上）
//! - `footer`：底部区域（用于 actions 等）
//! - `bordered` / `borderless`：边框变体
//! - `hoverable`：悬浮提升（shadow 效果）
//! - `size`：尺寸变体（Small/Medium/Large，默认 Medium，通过 Sizable trait）
//!
//! RML `<Card>` 编译为 `rml_ui::Card::new(("rml_el", N)).<setters>...`：
//! - `title="..."` / `title={expr}` → `.title(...)`
//! - `extra={expr}` → `.extra(expr)`
//! - `cover={expr}` → `.cover(expr)`
//! - `footer={expr}` → `.footer(expr)`
//! - `borderless=""` → `.borderless()` / `bordered={expr}` → `.bordered(expr)`
//! - `hoverable=""` → `.hoverable(true)` / `hoverable={expr}` → `.hoverable(expr)`
//! - body 子节点 → `.child(...)`

use gpui::prelude::FluentBuilder as _;
use gpui::{
    div, point, px, AnyElement, App, BoxShadow, Div, ElementId, Hsla, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{h_flex, ActiveTheme, Sizable, Size, StyledExt};
use rust_rml_core::theme::color as theme_color;

fn themed_hsla(name: &str, fallback: Hsla) -> Hsla {
    let c = theme_color(name);
    if c.a > 0.0 {
        c.into()
    } else {
        fallback
    }
}

/// 卡片变体
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
pub enum CardVariant {
    /// 默认带边框卡片
    #[default]
    Default,
    /// 无边框卡片
    Borderless,
}

/// 卡片组件 —— 参考 Ant Design Card 标准封装
///
/// 布局：`[cover] → [header: title | extra] → [body: children] → [footer]`
#[derive(IntoElement)]
pub struct Card {
    base: Div,
    id: ElementId,
    title: Option<AnyElement>,
    extra: Option<AnyElement>,
    cover: Option<AnyElement>,
    footer: Option<AnyElement>,
    variant: CardVariant,
    size: Size,
    hoverable: bool,
    children: Vec<AnyElement>,
}

impl Card {
    /// 创建卡片。`id` 由 codegen 自动注入 `("rml_el", N)`，用户无需手写。
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            title: None,
            extra: None,
            cover: None,
            footer: None,
            variant: CardVariant::Default,
            size: Size::default(),
            hoverable: false,
            children: Vec::new(),
        }
    }

    /// 设置卡片标题（左上角）。接受任意元素，便于插入图标等。
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// 设置标题栏右侧附加区（如操作按钮）。
    pub fn extra(mut self, extra: impl IntoElement) -> Self {
        self.extra = Some(extra.into_any_element());
        self
    }

    /// 设置顶部封面图（位于标题之上）。
    pub fn cover(mut self, cover: impl IntoElement) -> Self {
        self.cover = Some(cover.into_any_element());
        self
    }

    /// 设置底部区域（如 actions 操作栏）。
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// 设置变体。
    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }

    /// 使用无边框变体。
    pub fn borderless(mut self) -> Self {
        self.variant = CardVariant::Borderless;
        self
    }

    /// 显式控制边框（Ant Design 兼容）。
    /// `true` → Default，`false` → Borderless。
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.variant = if bordered {
            CardVariant::Default
        } else {
            CardVariant::Borderless
        };
        self
    }

    /// 设置悬浮提升效果（shadow）。
    pub fn hoverable(mut self, hoverable: bool) -> Self {
        self.hoverable = hoverable;
        self
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Card {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Card {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Card {}

impl Sizable for Card {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let bg = themed_hsla("--card-bg", theme.background);
        let fg = theme.foreground;
        let border_color = themed_hsla("--card-border", theme.border);

        let size = self.size;
        let radius = match size {
            Size::Small | Size::XSmall => theme.radius,
            Size::Large => px(12.),
            Size::Medium | Size::Size(_) => theme.radius_lg,
        };
        let (body_px, body_py) = match size {
            Size::Small | Size::XSmall => (px(12.), px(12.)),
            Size::Large => (px(32.), px(32.)),
            Size::Medium | Size::Size(_) => (px(24.), px(24.)),
        };
        // AntD v5 Card head: min-height 48 (default) / 36 (small), padding 0,
        // content vertically centered. Using min_h avoids padding-on-top-of-text
        // height inflation (py(16) + line-height ≈ 52px > AntD's 48px).
        let (header_px, header_min_h, footer_py) = match size {
            Size::Small | Size::XSmall => (px(12.), px(36.), px(8.)),
            Size::Large => (px(32.), px(56.), px(20.)),
            Size::Medium | Size::Size(_) => (px(24.), px(48.), px(16.)),
        };

        let has_header = self.title.is_some() || self.extra.is_some();
        let has_body = !self.children.is_empty();

        let header: Option<AnyElement> = if has_header {
            Some(
                h_flex()
                    .items_center()
                    .justify_between()
                    .px(header_px)
                    .min_h(header_min_h)
                    .border_b_1()
                    .border_color(border_color)
                    .when_some(self.title, |this, title| {
                        this.child(div().text_sm().font_semibold().child(title))
                    })
                    .when_some(self.extra, |this, extra| this.child(extra))
                    .into_any_element(),
            )
        } else {
            None
        };

        let body: Option<AnyElement> = if has_body {
            Some(
                div()
                    .px(body_px)
                    .py(body_py)
                    .children(self.children)
                    .into_any_element(),
            )
        } else {
            None
        };

        let apply_border = self.variant == CardVariant::Default;
        let hoverable = self.hoverable;
        let shadow_enabled = theme.shadow;

        self.base
            .id(self.id)
            .flex()
            .flex_col()
            .bg(bg)
            .text_color(fg)
            .rounded(radius)
            .overflow_hidden()
            .when(apply_border, |this| {
                this.border_1().border_color(border_color)
            })
            .when(hoverable && shadow_enabled, |this| {
                let shadow_base = cx.theme().foreground;
                let transparent = cx.theme().transparent;
                let hover_shadow = vec![
                    BoxShadow {
                        color: shadow_base.opacity(0.08),
                        offset: point(px(0.), px(1.)),
                        blur_radius: px(2.),
                        spread_radius: px(-2.),
                        inset: false,
                    },
                    BoxShadow {
                        color: shadow_base.opacity(0.06),
                        offset: point(px(0.), px(3.)),
                        blur_radius: px(6.),
                        spread_radius: px(0.),
                        inset: false,
                    },
                    BoxShadow {
                        color: shadow_base.opacity(0.04),
                        offset: point(px(0.), px(5.)),
                        blur_radius: px(12.),
                        spread_radius: px(4.),
                        inset: false,
                    },
                ];
                this.hover(move |s| {
                    let s = if apply_border {
                        s.border_color(transparent)
                    } else {
                        s
                    };
                    s.shadow(hover_shadow.clone())
                })
            })
            .when_some(self.cover, |this, cover| this.child(cover))
            .when_some(header, |this, h| this.child(h))
            .when_some(body, |this, b| this.child(b))
            .when_some(self.footer, |this, footer| {
                this.child(
                    div()
                        .px(header_px)
                        .py(footer_py)
                        .border_t_1()
                        .border_color(border_color)
                        .child(footer),
                )
            })
    }
}

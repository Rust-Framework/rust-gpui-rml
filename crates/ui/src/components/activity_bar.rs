//! ActivityBar —— VS Code 风格左侧活动栏控件

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, px, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme, IconName,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use smallvec::SmallVec;

// ── Trait 定义 ──

/// 活动栏面板项接口（S1–S3）
pub trait IActivityPanel: Send + Sync + 'static {
    fn id(&self) -> SharedString;
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    fn is_activated(&self) -> bool;
    /// 面板内容元素。默认返回 `None`，面板内容通常通过 ActivityBar 的子元素提供。
    fn panel(&self) -> Option<AnyElement> {
        None
    }
}

/// 活动栏底部动作项接口（B1–B2）
pub trait IActivityAct: Send + Sync + 'static {
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    fn on_click(&self, window: &mut Window, cx: &mut App);
}

// ── 类型别名（用于 #[computed] 返回类型） ──

pub type ActivityPanels = Vec<Arc<dyn IActivityPanel>>;
pub type ActivityActs = Vec<Arc<dyn IActivityAct>>;

// ── 默认实现 ──

/// 活动栏面板项默认实现（元数据；面板内容由 ActivityBar RML 子节点承载）
pub struct ActivityPanel {
    id: SharedString,
    icon: IconName,
    title: SharedString,
    active: bool,
}

impl ActivityPanel {
    pub fn new(id: impl Into<SharedString>, icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon,
            title: title.into(),
            active: false,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn into_arc(self) -> Arc<dyn IActivityPanel> {
        Arc::new(self)
    }
}

impl IActivityPanel for ActivityPanel {
    fn id(&self) -> SharedString {
        self.id.clone()
    }

    fn icon(&self) -> IconName {
        self.icon.clone()
    }

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn is_activated(&self) -> bool {
        self.active
    }
}

/// 活动栏底部动作项默认实现
pub struct ActivityAct {
    icon: IconName,
    title: SharedString,
    on_click: Option<Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>>,
}

impl ActivityAct {
    pub fn new(icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            icon,
            title: title.into(),
            on_click: None,
        }
    }

    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(f));
        self
    }

    pub fn into_arc(self) -> Arc<dyn IActivityAct> {
        Arc::new(self)
    }
}

impl IActivityAct for ActivityAct {
    fn icon(&self) -> IconName {
        self.icon.clone()
    }

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn on_click(&self, window: &mut Window, cx: &mut App) {
        if let Some(f) = &self.on_click {
            f(window, cx);
        }
    }
}

// ── ActivityBar 组件 ──

/// ActivityBar 活动栏控件
#[derive(IntoElement)]
pub struct ActivityBar {
    id: ElementId,
    bar_width: gpui::Pixels,
    panels: ActivityPanels,
    actions: ActivityActs,
    /// 当前激活面板 ID（空字符串表示全部折叠）。优先于 `panels[].is_activated()` 判定侧栏显隐。
    active_panel_id: Option<SharedString>,
    on_panel_change: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    panel_children: SmallVec<[AnyElement; 2]>,
}

impl ActivityBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bar_width: px(48.),
            panels: Vec::new(),
            actions: Vec::new(),
            active_panel_id: None,
            on_panel_change: None,
            panel_children: SmallVec::new(),
        }
    }

    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.bar_width = width;
        self
    }

    pub fn panels(mut self, panels: ActivityPanels) -> Self {
        self.panels = panels;
        self
    }

    pub fn actions(mut self, actions: ActivityActs) -> Self {
        self.actions = actions;
        self
    }

    pub fn active_panel_id(mut self, id: impl Into<SharedString>) -> Self {
        self.active_panel_id = Some(id.into());
        self
    }

    pub fn on_panel_change(
        mut self,
        f: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_panel_change = Some(Rc::new(f));
        self
    }
}

impl ParentElement for ActivityBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.panel_children.extend(elements);
    }
}

impl RenderOnce for ActivityBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_panel_change = self.on_panel_change.clone();
        // 侧栏显隐：优先看 active_panel_id，其次看 panels 元数据中的 is_activated
        let any_active = self
            .active_panel_id
            .as_ref()
            .map(|id| !id.is_empty())
            .unwrap_or_else(|| self.panels.iter().any(|p| p.is_activated()));
        let mut panel_buttons: SmallVec<[AnyElement; 4]> = SmallVec::new();
        for (ix, panel) in self.panels.iter().enumerate() {
            let id = panel.id();
            let icon = panel.icon();
            let title = panel.title();
            let active = self
                .active_panel_id
                .as_ref()
                .map(|active_id| active_id == &id)
                .unwrap_or_else(|| panel.is_activated());
            let on_change = on_panel_change.clone();

            panel_buttons.push(
                Button::new(("activity-panel", ix))
                    .ghost()
                    .icon(icon)
                    .tooltip(title)
                    .h(px(36.))
                    .w(px(36.))
                    .my(px(2.))
                    .when(active, |btn| btn.bg(cx.theme().sidebar_accent))
                    .on_click(move |_, window, cx| {
                        if let Some(f) = &on_change {
                            f(&id, window, cx);
                        }
                    })
                    .into_any_element(),
            );
        }

        let action_buttons: SmallVec<[AnyElement; 4]> = self
            .actions
            .iter()
            .enumerate()
            .map(|(ix, action)| {
                let action = action.clone();
                Button::new(("activity-action", ix))
                    .ghost()
                    .icon(action.icon())
                    .tooltip(action.title())
                    .w(self.bar_width)
                    .h(px(48.))
                    .on_click(move |_, window, cx| action.on_click(window, cx))
                    .into_any_element()
            })
            .collect();

        let icon_bar = v_flex()
            .w(self.bar_width)
            .h_full()
            .flex_shrink_0()
            .justify_between()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(v_flex().w_full().items_center().children(panel_buttons))
            .child(v_flex().w_full().items_center().children(action_buttons));

        let mut row = h_flex().id(self.id).h_full().child(icon_bar);

        if any_active && !self.panel_children.is_empty() {
            row = row.child(
                gpui::div()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .overflow_hidden()
                    .children(self.panel_children),
            );
        }

        row
    }
}

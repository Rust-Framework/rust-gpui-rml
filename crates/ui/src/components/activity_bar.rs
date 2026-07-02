//! ActivityBar —— VS Code 风格左侧活动栏（单 Entity 模型）
//!
//! 架构：
//! - 单 `ActivityBar` Entity：同时渲染图标栏 + 面板内容
//! - `set_active_id` 直接修改字段 + `cx.notify()` 触发自身重渲
//! - 无 EventEmitter、无 SidePanel、无 Shell
//!
//! RML 用法：`<ActivityBar ref="activity_bar" />`
//! Host 在 `on_loaded` 中 `cx.new(|_| ActivityBar::new(panels))` 创建并激活首项。

use std::sync::Arc;

use gpui::{
    AnyElement, App, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window,
    div, px, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme, IconName,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use smallvec::SmallVec;

// ── Trait 定义 ──

/// 活动栏面板项接口
pub trait IActivityPanel: Send + Sync + 'static {
    fn id(&self) -> SharedString;
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    /// 面板内容。`ActivityBar` 在渲染时调用当前激活面板的 `panel`。
    fn panel(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let _ = (window, cx);
        None
    }
}

/// 活动栏底部动作项接口
pub trait IActivityAct: Send + Sync + 'static {
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    fn on_click(&self, window: &mut Window, cx: &mut App);
}

pub type ActivityPanels = Vec<Arc<dyn IActivityPanel>>;
pub type ActivityActs = Vec<Arc<dyn IActivityAct>>;

// ── 默认实现 ──

/// 活动栏面板项（纯元数据，无 `panel` 内容）
pub struct ActivityPanel {
    id: SharedString,
    icon: IconName,
    title: SharedString,
}

impl ActivityPanel {
    pub fn new(
        id: impl Into<SharedString>,
        icon: IconName,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            title: title.into(),
        }
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
}

/// 活动栏底部动作项
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

    pub fn on_click(
        mut self,
        f: impl Fn(&mut Window, &mut App) + Send + Sync + 'static,
    ) -> Self {
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

// ── ActivityBar Entity（单 Entity：图标栏 + 面板内容） ──

/// ActivityBar：单 Entity 同时渲染图标栏 + 面板内容。
///
/// `set_active_id` 直接修改字段 + `cx.notify()` 触发自身重渲，无需事件订阅。
/// Host 在 `on_loaded` 中 `cx.new(|_| ActivityBar::new(panels))` 创建并调用
/// `activate_first` 激活首项。
pub struct ActivityBar {
    panels: ActivityPanels,
    actions: ActivityActs,
    active_id: Option<SharedString>,
    bar_width: gpui::Pixels,
}

impl ActivityBar {
    pub fn new(panels: ActivityPanels) -> Self {
        Self {
            panels,
            actions: Vec::new(),
            active_id: None,
            bar_width: px(48.),
        }
    }

    /// 激活首个面板。Host 在 `on_loaded` 中创建 Entity 后调用。
    pub fn activate_first(&mut self, cx: &mut Context<Self>) {
        if let Some(first) = self.panels.first() {
            self.set_active_id(Some(first.id()), cx);
        }
    }

    pub fn set_panels(&mut self, panels: ActivityPanels, cx: &mut Context<Self>) {
        self.panels = panels;
        cx.notify();
    }

    pub fn set_actions(&mut self, actions: ActivityActs, cx: &mut Context<Self>) {
        self.actions = actions;
        cx.notify();
    }

    pub fn set_active_id(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        if self.active_id == id {
            return;
        }
        self.active_id = id;
        cx.notify();
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
    }
}

impl Render for ActivityBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_id = self.active_id.clone();

        // ── 图标栏 ──
        let mut panel_buttons: SmallVec<[AnyElement; 4]> = SmallVec::new();
        for (ix, panel) in self.panels.iter().enumerate() {
            let id = panel.id();
            let icon = panel.icon();
            let title = panel.title();
            let active = active_id.as_ref() == Some(&id);

            panel_buttons.push(
                Button::new(("activity-panel", ix))
                    .ghost()
                    .icon(icon)
                    .tooltip(title)
                    .h(px(36.))
                    .w(px(36.))
                    .my(px(2.))
                    .when(active, |btn| btn.bg(cx.theme().sidebar_accent))
                    .on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {
                        let new_id = if this.active_id.as_ref() == Some(&id) {
                            None
                        } else {
                            Some(id.clone())
                        };
                        this.set_active_id(new_id, cx);
                    }))
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
                    .h(px(36.))
                    .w(px(36.))
                    .my(px(2.))
                    .on_click(move |_, window, cx| action.on_click(window, cx))
                    .into_any_element()
            })
            .collect();

        let bar = v_flex()
            .w(self.bar_width)
            .h_full()
            .flex_shrink_0()
            .justify_between()
            .bg(cx.theme().title_bar)
            .child(v_flex().w_full().items_center().children(panel_buttons))
            .child(v_flex().w_full().items_center().children(action_buttons));

        // ── 面板内容 ──
        // panel_body 背景用 title_bar：亮色主题接近白色，暗色主题比窗口主色略亮形成色差。
        // icon_bar 背景透明，与窗口背景一致；两者之间无边框线。
        let active_id_for_body = self.active_id.clone();
        let panel_body = if active_id_for_body.is_some() {
            let body = self
                .panels
                .iter()
                .find(|p| p.id() == active_id_for_body.as_deref().unwrap_or(""))
                .and_then(|panel| panel.panel(window, cx));
            match body {
                Some(body) => div()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .overflow_hidden()
                    .child(body)
                    .into_any_element(),
                None => div().w_0().h_full().into_any_element(),
            }
        } else {
            div().w_0().h_full().into_any_element()
        };

        h_flex().size_full().child(bar).child(panel_body)
    }
}

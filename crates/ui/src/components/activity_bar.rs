//! ActivityBar —— VS Code 风格左侧活动栏（双 Entity 事件驱动模型）
//!
//! 架构：
//! - `ActivityBar` Entity：仅渲染图标栏，持有激活态，点击时 emit `ActivityBarEvent`
//! - `ActivitySidePanel` Entity：仅渲染面板内容，由 Host 通过 `set_active_id` 驱动
//! - `ActivityBarShell` RenderOnce：布局包装器，将两个 Entity 水平排列
//! - Host（如 MainWindow）订阅 `ActivityBarEvent` → 调用 `SidePanel::set_active_id`
//!
//! 设计要点（对齐参考实现 rust-agent-ide）：
//! 1. 所有副作用（Entity 创建、Global 修改）在构造器 / on_loaded 中完成，不在 render 中执行
//! 2. 激活态由 Entity 字段管理，非全局静态状态
//! 3. `flush_pending` 延迟回调：`set_active_id` 只写 pending，render 开头执行

use std::sync::Arc;

use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, IntoElement, ParentElement, Render,
    RenderOnce, SharedString, Styled, Window, div, px, prelude::FluentBuilder as _,
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
    /// 面板内容。`ActivitySidePanel` 在渲染时调用当前激活面板的 `panel`。
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

// ── 事件 ──

#[derive(Clone)]
pub enum ActivityBarEvent {
    ItemActivated(SharedString),
    ItemDeactivated(SharedString),
}

// ── 默认实现 ──

/// 活动栏面板项（纯元数据，无 `panel` 内容）
pub struct ActivityPanel {
    id: SharedString,
    icon: IconName,
    title: SharedString,
}

impl ActivityPanel {
    pub fn new(id: impl Into<SharedString>, icon: IconName, title: impl Into<SharedString>) -> Self {
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

// ── ActivityBar Entity（图标栏） ──

/// ActivityBar：仅渲染图标栏，持有激活态，点击时 emit 事件。
///
/// Host（如 MainWindow）订阅 `ActivityBarEvent` 并联动 `ActivitySidePanel`。
pub struct ActivityBar {
    panels: ActivityPanels,
    actions: ActivityActs,
    active_id: Option<SharedString>,
    bar_width: gpui::Pixels,
}

impl EventEmitter<ActivityBarEvent> for ActivityBar {}

impl ActivityBar {
    /// 构造图标栏。**不**在此激活首项 —— 此时 Host 尚未 `cx.subscribe`，
    /// emit 的事件会丢失。Host 应在 subscribe 之后调用 [`activate_first`]。
    pub fn new(panels: ActivityPanels) -> Self {
        Self {
            panels,
            actions: Vec::new(),
            active_id: None,
            bar_width: px(48.),
        }
    }

    /// 激活首个面板。Host 须在 `cx.subscribe(&bar, ...).detach()` 之后调用，
    /// 以保证 `ItemActivated` 事件能被订阅者收到。
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
        match (&self.active_id, &id) {
            (Some(old), Some(new)) if old != new => {
                cx.emit(ActivityBarEvent::ItemDeactivated(old.clone()));
                cx.emit(ActivityBarEvent::ItemActivated(new.clone()));
            }
            (Some(old), None) => cx.emit(ActivityBarEvent::ItemDeactivated(old.clone())),
            (None, Some(new)) => cx.emit(ActivityBarEvent::ItemActivated(new.clone())),
            _ => {}
        }
        self.active_id = id;
        cx.notify();
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
    }
}

impl Render for ActivityBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_id = self.active_id.clone();

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

        v_flex()
            .w(self.bar_width)
            .h_full()
            .flex_shrink_0()
            .justify_between()
            .bg(cx.theme().sidebar)
            .child(v_flex().w_full().items_center().children(panel_buttons))
            .child(v_flex().w_full().items_center().children(action_buttons))
    }
}

// ── ActivitySidePanel Entity（面板内容） ──

/// ActivitySidePanel：仅渲染当前激活面板的内容。
///
/// 通过 `set_active_id` 驱动，由 Host 在收到 `ActivityBarEvent` 后调用。
pub struct ActivitySidePanel {
    panels: ActivityPanels,
    active_id: Option<SharedString>,
    pending_deactivate: Option<SharedString>,
    pending_activate: Option<SharedString>,
}

impl ActivitySidePanel {
    pub fn new(panels: ActivityPanels) -> Self {
        Self {
            panels,
            active_id: None,
            pending_deactivate: None,
            pending_activate: None,
        }
    }

    pub fn set_panels(&mut self, panels: ActivityPanels, cx: &mut Context<Self>) {
        self.panels = panels;
        cx.notify();
    }

    pub fn set_active_id(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        if self.active_id == id {
            return;
        }
        self.pending_deactivate = self.active_id.clone();
        self.active_id = id.clone();
        self.pending_activate = id;
        cx.notify();
    }

    fn flush_pending(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // 预留：未来可在 IActivityPanel 上添加 activate/deactivate 生命周期钩子
        self.pending_deactivate = None;
        self.pending_activate = None;
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
    }
}

impl Render for ActivitySidePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.flush_pending(window, cx);

        let has_active = self.active_id.is_some();
        if !has_active {
            return div().w_0().h_full().into_any_element();
        }

        let active_id = self.active_id.clone();
        let body = self
            .panels
            .iter()
            .find(|p| p.id() == active_id.as_deref().unwrap_or(""))
            .and_then(|panel| panel.panel(window, cx));

        match body {
            Some(body) => div()
                .flex_1()
                .h_full()
                .min_w_0()
                .overflow_hidden()
                .bg(cx.theme().sidebar)
                .child(body)
                .into_any_element(),
            None => div().w_0().h_full().into_any_element(),
        }
    }
}

// ── ActivityBarShell RenderOnce（布局包装器） ──

/// ActivityBarShell：将 `ActivityBar` 和 `ActivitySidePanel` 水平排列。
///
/// RML 用法：`<ActivityBarShell bar={activity_bar} panel={side_panel} />`
#[derive(IntoElement)]
pub struct ActivityBarShell {
    bar: Option<Entity<ActivityBar>>,
    panel: Option<Entity<ActivitySidePanel>>,
}

impl Default for ActivityBarShell {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityBarShell {
    pub fn new() -> Self {
        Self {
            bar: None,
            panel: None,
        }
    }

    pub fn bar(mut self, bar: Entity<ActivityBar>) -> Self {
        self.bar = Some(bar);
        self
    }

    pub fn panel(mut self, panel: Entity<ActivitySidePanel>) -> Self {
        self.panel = Some(panel);
        self
    }
}

impl RenderOnce for ActivityBarShell {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut row = h_flex().h_full();
        if let Some(bar) = self.bar {
            row = row.child(bar);
        }
        if let Some(panel) = self.panel {
            row = row.child(panel);
        }
        row
    }
}

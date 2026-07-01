//! ActivityBar —— VS Code 风格左侧活动栏（自治理：激活态 + 面板内容）

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

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

/// 活动栏面板项接口
pub trait IActivityPanel: Send + Sync + 'static {
    fn id(&self) -> SharedString;
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    /// 是否激活（仅元数据面板；ActivityBar 内部状态优先）
    fn is_activated(&self) -> bool;
    /// 面板内容。ActivityBar 在渲染时调用当前激活面板的 `panel`。
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

type ActiveState = Arc<Mutex<Option<SharedString>>>;

fn active_state_for_bar(bar_id: &ElementId) -> ActiveState {
    static STATES: OnceLock<Mutex<HashMap<String, ActiveState>>> = OnceLock::new();
    let key = format!("{bar_id:?}");
    STATES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("activity bar state lock")
        .entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(None)))
        .clone()
}

// ── 默认实现 ──

/// 活动栏面板项（纯元数据，无 `panel` 内容）
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

// ── ActivityBar 组件 ──

/// ActivityBar：图标栏 + 激活面板内容（内容来自 `IActivityPanel::panel`）
#[derive(IntoElement)]
pub struct ActivityBar {
    id: ElementId,
    bar_width: gpui::Pixels,
    panels: ActivityPanels,
    actions: ActivityActs,
    active_state: ActiveState,
    on_panel_change: Option<Arc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    #[allow(dead_code)]
    panel_children: SmallVec<[AnyElement; 2]>,
}

impl ActivityBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            active_state: active_state_for_bar(&id),
            id,
            bar_width: px(48.),
            panels: Vec::new(),
            actions: Vec::new(),
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

    pub fn on_panel_change(
        mut self,
        f: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_panel_change = Some(Arc::new(f));
        self
    }
}

impl ParentElement for ActivityBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.panel_children.extend(elements);
    }
}

impl RenderOnce for ActivityBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_panel_change = self.on_panel_change.clone();
        let active_state = self.active_state.clone();

        {
            let mut active = active_state.lock().expect("activity bar active lock");
            if active.is_none() {
                *active = self.panels.first().map(|p| p.id());
            }
        }

        let active_id = active_state.lock().expect("activity bar active lock").clone();
        let any_active = active_id.as_ref().is_some_and(|id| !id.is_empty());

        let mut panel_buttons: SmallVec<[AnyElement; 4]> = SmallVec::new();
        for (ix, panel) in self.panels.iter().enumerate() {
            let id = panel.id();
            let icon = panel.icon();
            let title = panel.title();
            let active = active_id.as_ref() == Some(&id);
            let on_change = on_panel_change.clone();
            let state = active_state.clone();

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
                        {
                            let mut current = state.lock().expect("activity bar active lock");
                            if current.as_ref() == Some(&id) {
                                *current = None;
                            } else {
                                *current = Some(id.clone());
                            }
                        }
                        if let Some(f) = &on_change {
                            f(&id, window, cx);
                        }
                        window.refresh();
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
                    .h(px(36.))
                    .w(px(36.))
                    .my(px(2.))
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
            .child(v_flex().w_full().items_center().children(panel_buttons))
            .child(v_flex().w_full().items_center().children(action_buttons));

        let mut row = h_flex().id(self.id).h_full().child(icon_bar);

        if any_active {
            if let Some(aid) = active_id.as_ref() {
                if let Some(panel) = self.panels.iter().find(|p| &p.id() == aid) {
                    if let Some(body) = panel.panel(window, cx) {
                        row = row.child(
                            gpui::div()
                                .flex_1()
                                .h_full()
                                .min_w_0()
                                .overflow_hidden()
                                .bg(cx.theme().sidebar)
                                .child(body),
                        );
                    }
                }
            }
        }

        row
    }
}

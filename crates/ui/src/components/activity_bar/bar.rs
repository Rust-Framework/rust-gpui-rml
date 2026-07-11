//! `ActivityBar` —— VS Code 风格左侧活动栏单 Entity

use std::sync::Arc;

use gpui::{
    AnyElement, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window,
    div, px, prelude::FluentBuilder as _,
};
use gpui_component::{ActiveTheme, button::{Button, ButtonVariants as _}, h_flex, v_flex};
use rml_core::command::CallContext;
use smallvec::SmallVec;

use super::icon::resolve_icon;
use super::traits::{IActivityAct, IActivityPanel};

/// ActivityBar：单 Entity 同时渲染图标栏 + 面板内容。
///
/// `set_active_id` 直接修改字段 + `cx.notify()` 触发自身重渲，无需事件订阅。
/// Host 在 `on_loaded` 中 `cx.new(|_| ActivityBar::new(panels))` 创建并调用
/// `activate_first` 激活首项。
pub struct ActivityBar {
    panels: Vec<Arc<dyn IActivityPanel>>,
    actions: Vec<Arc<dyn IActivityAct>>,
    active_id: Option<SharedString>,
    bar_width: gpui::Pixels,
}

impl ActivityBar {
    pub fn new(panels: Vec<Arc<dyn IActivityPanel>>) -> Self {
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
            self.set_active_id(Some(first.id().to_string().into()), cx);
        }
    }

    pub fn set_panels(&mut self, panels: Vec<Arc<dyn IActivityPanel>>, cx: &mut Context<Self>) {
        self.panels = panels;
        cx.notify();
    }

    pub fn set_actions(&mut self, actions: Vec<Arc<dyn IActivityAct>>, cx: &mut Context<Self>) {
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
            let id: SharedString = panel.id().to_string().into();
            let icon = resolve_icon(panel.icon(), window);
            let title = panel.name();
            let active = active_id.as_ref() == Some(&id);

            panel_buttons.push(
                Button::new(("activity-panel", ix))
                    .ghost()
                    .child(icon)
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
                    .child(resolve_icon(action.icon(), window))
                    .tooltip(action.name())
                    .h(px(36.))
                    .w(px(36.))
                    .my(px(2.))
                    .on_click(move |_, window, cx| {
                        action.execute(&mut CallContext::new(window, cx));
                    })
                    .into_any_element()
            })
            .collect();

        let bar = v_flex()
            .w(self.bar_width)
            .h_full()
            .flex_shrink_0()
            .justify_between()
            .bg(cx.theme().sidebar)
            .child(v_flex().w_full().items_center().children(panel_buttons))
            .child(v_flex().w_full().items_center().children(action_buttons));

        // ── 面板内容 ──
        // icon_bar 使用 sidebar（chrome）；panel_body 使用 tab_active（editor 工作面），
        // 与 Tab Body / 激活 Tab 同色，并与图标栏形成可识别色差。
        let active_id_for_body = self.active_id.clone();
        let panel_body = if let Some(active) = active_id_for_body.as_deref() {
            match self.panels.iter().find(|p| p.id() == active) {
                Some(panel) => div()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .overflow_hidden()
                    .bg(cx.theme().tab_active)
                    .child(panel.render(window, cx))
                    .into_any_element(),
                None => div().w_0().h_full().into_any_element(),
            }
        } else {
            div().w_0().h_full().into_any_element()
        };

        h_flex().size_full().child(bar).child(panel_body)
    }
}

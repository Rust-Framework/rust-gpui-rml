//! ActivityBar —— VS Code 风格左侧活动栏控件

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, px, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme, IconName,
    button::{Button, ButtonVariants as _},
    menu::DropdownMenu as _,
    h_flex, v_flex,
};
use smallvec::SmallVec;

use crate::window::menu_bar::build_popup_menu;
use crate::window::types::MenuItem;

/// 活动栏面板项（S1–S3）
pub struct ActivityPanelItem {
    pub id: SharedString,
    pub icon: IconName,
    pub title: SharedString,
    pub panel: Option<AnyElement>,
    pub active: bool,
}

impl ActivityPanelItem {
    pub fn new(id: impl Into<SharedString>, icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon,
            title: title.into(),
            panel: None,
            active: false,
        }
    }

    pub fn panel(mut self, element: impl IntoElement) -> Self {
        self.panel = Some(element.into_any_element());
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl Clone for ActivityPanelItem {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            icon: self.icon.clone(),
            title: self.title.clone(),
            panel: None,
            active: self.active,
        }
    }
}

/// 活动栏底部动作项（B1–B2）
#[derive(Clone)]
pub struct ActivityActionItem {
    pub icon: IconName,
    pub title: SharedString,
    pub on_click: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    pub context_menu: Vec<MenuItem>,
}

impl ActivityActionItem {
    pub fn new(icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            icon,
            title: title.into(),
            on_click: None,
            context_menu: Vec::new(),
        }
    }

    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }

    pub fn context_menu(mut self, items: Vec<MenuItem>) -> Self {
        self.context_menu = items;
        self
    }
}

/// ActivityBar 活动栏控件
#[derive(IntoElement)]
pub struct ActivityBar {
    id: ElementId,
    bar_width: gpui::Pixels,
    panels: Vec<ActivityPanelItem>,
    actions: Vec<ActivityActionItem>,
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
            on_panel_change: None,
            panel_children: SmallVec::new(),
        }
    }

    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.bar_width = width;
        self
    }

    pub fn panels(mut self, panels: Vec<ActivityPanelItem>) -> Self {
        self.panels = panels;
        self
    }

    pub fn actions(mut self, actions: Vec<ActivityActionItem>) -> Self {
        self.actions = actions;
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_panel_change = self.on_panel_change.clone();
        let mut active_panel = None;
        let mut panel_fallback = (!self.panel_children.is_empty()).then(|| {
            gpui::div()
                .size_full()
                .children(self.panel_children)
                .into_any_element()
        });

        let mut panel_buttons: SmallVec<[AnyElement; 4]> = SmallVec::new();
        for (ix, panel) in self.panels.into_iter().enumerate() {
            let id = panel.id.clone();
            let icon = panel.icon;
            let title = panel.title.clone();
            let active = panel.active;
            if active {
                active_panel = panel.panel.or_else(|| panel_fallback.take());
            }
            let on_change = on_panel_change.clone();

            panel_buttons.push(
                Button::new(("activity-panel", ix))
                    .ghost()
                    .icon(icon)
                    .tooltip(title)
                    .w(self.bar_width)
                    .h(px(48.))
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
                let on_click = action.on_click.clone();
                let menu_items = action.context_menu.clone();
                let mut btn = Button::new(("activity-action", ix))
                    .ghost()
                    .icon(action.icon.clone())
                    .tooltip(action.title.clone())
                    .w(self.bar_width)
                    .h(px(48.));

                if menu_items.is_empty() {
                    if let Some(f) = on_click {
                        btn = btn.on_click(move |_, window, cx| f(window, cx));
                    }
                    btn.into_any_element()
                } else {
                    btn.dropdown_menu(move |menu, window, cx| {
                        build_popup_menu(menu, &menu_items, window, cx)
                    })
                    .into_any_element()
                }
            })
            .collect();

        h_flex()
            .id(self.id)
            .h_full()
            .child(
                v_flex()
                    .w(self.bar_width)
                    .h_full()
                    .flex_shrink_0()
                    .justify_between()
                    .bg(cx.theme().sidebar)
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(v_flex().w_full().items_center().children(panel_buttons))
                    .child(v_flex().w_full().items_center().children(action_buttons)),
            )
            .child(
                gpui::div()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .overflow_hidden()
                    .when_some(active_panel, |this, panel| this.child(panel)),
            )
    }
}

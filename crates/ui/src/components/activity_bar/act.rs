//! `ActivityAct` —— 活动栏底部动作项默认实现

use std::sync::Arc;

use gpui::{App, SharedString, Window};
use gpui_component::IconName;

use super::traits::IActivityAct;

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

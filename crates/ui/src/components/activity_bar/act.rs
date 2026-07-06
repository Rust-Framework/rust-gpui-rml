//! `ActivityAct` —— 活动栏底部动作项默认实现

use std::sync::Arc;

use gpui::{App, SharedString, Window};
use rml_core::command::{CallContext, ICommand};
use rml_core::contribution::{IContribution, IconSpec};

use super::traits::IActivityAct;

type ActClickHandler = Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>;

/// 活动栏底部动作项
pub struct ActivityAct {
    id: String,
    icon: IconSpec,
    title: SharedString,
    on_click: Option<ActClickHandler>,
}

impl ActivityAct {
    pub fn new(
        id: impl Into<String>,
        icon: impl Into<IconSpec>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
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

impl IContribution for ActivityAct {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> SharedString {
        self.title.clone()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(self.icon.clone())
    }
}

impl ICommand for ActivityAct {
    fn execute(&self, ctx: &mut CallContext) {
        if let Some(f) = &self.on_click {
            f(ctx.window, ctx.app);
        }
    }
}

impl IActivityAct for ActivityAct {}

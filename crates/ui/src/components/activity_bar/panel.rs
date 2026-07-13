//! `ActivityPanel` —— 活动栏面板项默认实现（纯元数据，`render` 返回空内容）

use std::sync::Arc;

use gpui::{AnyElement, App, IntoElement, SharedString, Window};
use rml_core::contribution::{IContribution, IVisual, IconSpec};

use super::traits::IActivityPanel;

/// 活动栏面板项（纯元数据，`render` 返回空 div）
pub struct ActivityPanel {
    id: String,
    icon: IconSpec,
    title: SharedString,
}

impl ActivityPanel {
    pub fn new(
        id: impl Into<String>,
        icon: impl Into<IconSpec>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            title: title.into(),
        }
    }

    pub fn into_arc(self) -> Arc<dyn IActivityPanel> {
        Arc::new(self)
    }
}

impl IContribution for ActivityPanel {
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

impl IVisual for ActivityPanel {
    fn render(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        gpui::div().into_any_element()
    }
}

impl IActivityPanel for ActivityPanel {}

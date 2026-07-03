//! `ActivityPanel` —— 活动栏面板项默认实现（纯元数据，无 `panel` 内容）

use std::sync::Arc;

use gpui::SharedString;
use gpui_component::IconName;

use super::traits::IActivityPanel;

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

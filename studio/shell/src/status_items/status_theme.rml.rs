//! 状态栏主题指示项 —— 右侧显示当前主题名。

use gpui::{SharedString, Window};
use rml::prelude::*;
use rml_core::contribution::IContribution;
use rml_core::i18n::t_static;
use rml_core::theme::ThemeExt;

#[contribute(
    host_id = "studio.shell",
    id = "status.theme",
    kind = "status",
    align = "right",
    order = 12
)]
#[component]
#[derive(Default)]
pub struct StatusTheme {
    theme: SharedString,
}

impl IContribution for StatusTheme {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.status.theme")
    }
}

impl ILifecycle for StatusTheme {
    fn before_render(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.theme = format!("{}", cx.current_theme()).into();
    }
}

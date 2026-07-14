//! 状态栏主题指示项 —— 右侧显示当前主题名。

use gpui::{AnyElement, ParentElement, SharedString, Styled};
use rml::prelude::*;
use rml_core::contribution::{IContribution, IVisual};
use rml_core::i18n::t_static;
use rml_core::theme::ThemeExt;

#[contribute(
    host_id = "studio.shell",
    id = "status.theme",
    kind = "status",
    align = "right",
    order = 12
)]
#[derive(Default)]
pub struct StatusTheme;

impl IContribution for StatusTheme {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.status.theme")
    }
}

impl IVisual for StatusTheme {
    fn render(&self, _window: &mut gpui::Window, cx: &mut gpui::App) -> AnyElement {
        let theme = cx.current_theme();
        gpui::div()
            .text_xs()
            .child(SharedString::from(format!(
                "{}: {}",
                t_static("studio.status.theme"),
                theme
            )))
            .into_any_element()
    }
}

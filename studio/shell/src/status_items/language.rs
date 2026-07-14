//! 状态栏语言指示项 —— 右侧显示当前 locale。

use gpui::{AnyElement, ParentElement, SharedString, Styled};
use rml::prelude::*;
use rml_core::contribution::{IContribution, IVisual};
use rml_core::i18n::{t_static, I18nExt};

#[contribute(
    host_id = "studio.shell",
    id = "status.language",
    kind = "status",
    align = "right",
    order = 11
)]
#[derive(Default)]
pub struct StatusLanguage;

impl IContribution for StatusLanguage {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.status.language")
    }
}

impl IVisual for StatusLanguage {
    fn render(&self, _window: &mut gpui::Window, cx: &mut gpui::App) -> AnyElement {
        let locale = cx.current_locale();
        gpui::div()
            .text_xs()
            .child(SharedString::from(format!(
                "{}: {}",
                t_static("studio.status.language"),
                locale
            )))
            .into_any_element()
    }
}

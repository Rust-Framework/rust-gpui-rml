//! 状态栏"就绪"指示项 —— 左侧默认状态文本。

use gpui::{AnyElement, ParentElement, SharedString, Styled};
use rml::prelude::*;
use rml_core::contribution::{IContribution, IVisual};
use rml_core::i18n::t_static;

#[contribute(host_id = "studio.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IContribution for StatusReady {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.status.ready")
    }
}

impl IVisual for StatusReady {
    fn render(&self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> AnyElement {
        gpui::div()
            .text_xs()
            .child(t_static("studio.status.ready"))
            .into_any_element()
    }
}

//! 状态栏编码指示项 —— 右侧显示文件编码。

use gpui::{AnyElement, ParentElement, SharedString, Styled};
use rml::prelude::*;
use rml_core::contribution::{IContribution, IVisual};
use rml_core::i18n::t_static;

#[contribute(
    host_id = "studio.shell",
    id = "status.encoding",
    kind = "status",
    align = "right",
    order = 10
)]
#[derive(Default)]
pub struct StatusEncoding;

impl IContribution for StatusEncoding {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.status.encoding")
    }
}

impl IVisual for StatusEncoding {
    fn render(&self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> AnyElement {
        gpui::div()
            .text_xs()
            .child(t_static("studio.status.encoding"))
            .into_any_element()
    }
}

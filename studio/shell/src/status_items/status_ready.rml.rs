//! 状态栏"就绪"指示项 —— 左侧默认状态文本。

use gpui::SharedString;
use rml::prelude::*;
use rml_core::contribution::IContribution;
use rml_core::i18n::t_static;

#[contribute(host_id = "studio.shell", id = "status.ready", kind = "status", order = 0)]
#[component]
#[derive(Default)]
pub struct StatusReady {}

impl IContribution for StatusReady {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.status.ready")
    }
}

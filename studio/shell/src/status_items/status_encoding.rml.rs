//! 状态栏编码指示项 —— 右侧显示文件编码。

use gpui::SharedString;
use rml::prelude::*;
use rml_core::contribution::IContribution;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "studio.shell",
    id = "status.encoding",
    kind = "status",
    align = "right",
    order = 10
)]
#[component]
#[derive(Default)]
pub struct StatusEncoding {}

impl IContribution for StatusEncoding {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.status.encoding")
    }
}

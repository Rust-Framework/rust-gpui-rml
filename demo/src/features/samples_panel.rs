//! 活动栏「示例」面板贡献（纯元数据，面板内容由 Shell RML 声明）

use rml_ui::IconName;
use rml::prelude::*;

#[contribute(
    host = "demo.shell.activity-bar",
    id = "samples",
    name = "shell.samples",
    icon = IconName::BookOpen,
    mode = Panel,
    order = 0,
)]
#[derive(Default)]
pub struct SamplesPanel {}

use rml::prelude::*;

#[contribute(
    host_id = "demo.shell",
    id = "components.status_bar",
    name = "case.status_bar.title",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct StatusBarCase {}

impl ILifecycle for StatusBarCase {}

/// 状态栏贡献：演示 status slot（从 shell_meta.rs 迁入）
#[contribute(host_id = "demo.shell", id = "status.ready", name = "shell.status_ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

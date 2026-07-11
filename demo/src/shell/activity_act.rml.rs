//! SettingsAct —— 活动栏底部动作项（IActivityAct），点击打开系统配置对话框。
//!
//! `IActivityAct: ICommand: IContribution`——动作本身是命令贡献。
//! 注册方式：在 `init_activity_bar` 中经 `bar.set_actions(vec![...], cx)` 注册，
//! 不走 `#[contribute]` 宏（actions 不自动注册到 host）。

use std::sync::Arc;

use gpui::SharedString;
use rml_core::command::{CallContext, ICommand};
use rml_core::contribution::{IContribution, IconSpec};
use rml_core::i18n::t_static;
use rml_ui::IActivityAct;

use crate::shell::settings_dialog::SettingsDialog;

/// 系统配置动作项——活动栏底部 Settings 齿轮图标，点击打开 SettingsDialog。
pub struct SettingsAct;

impl SettingsAct {
    pub fn into_arc(self) -> Arc<dyn IActivityAct> {
        Arc::new(self)
    }
}

impl IContribution for SettingsAct {
    fn id(&self) -> &str {
        "system_settings"
    }
    fn name(&self) -> SharedString {
        t_static("shell.system_settings")
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("Settings"))
    }
}

impl ICommand for SettingsAct {
    fn execute(&self, ctx: &mut CallContext) {
        SettingsDialog::default().open(ctx.window, ctx.app);
    }
}

impl IActivityAct for SettingsAct {}

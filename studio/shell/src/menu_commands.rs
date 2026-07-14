//! 菜单命令贡献 —— File/View/Help 三组菜单 + 叶子命令，全部经 `#[contribute]` 注册到 `studio.shell`。
//!
//! 叶子命令实现 `IContribution` + `ICommand`，经 `as_command()` 查询；
//! submenu root 仅实现 `IContribution`（无命令），作为分组节点。
//! `MenuViewModel::from_contribution` 按 `kind="menu"` 过滤，按 `parent_id` 组织树。

use gpui::SharedString;
use rml::prelude::*;
use rml_core::command::{CallContext, ICommand};
use rml_core::i18n::t_static;

use crate::main_window::{MainWindow, MainWindowRef};

/// 经 `MainWindowRef` 单例查询 MainWindow entity，执行视图绑定操作。
fn with_main_window<F>(ctx: &mut CallContext, f: F)
where
    F: FnOnce(&mut MainWindow, &mut gpui::Context<MainWindow>),
{
    if let Some(mw_ref) = ctx.app.get_service::<MainWindowRef>() {
        let _ = mw_ref.0.update(&mut *ctx.app, |this, cx| f(this, cx));
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Submenu root（分组节点，无命令）
// ──────────────────────────────────────────────────────────────────────────

#[contribute(host_id = "studio.shell", id = "menu.file", kind = "menu", order = 1, label = "studio.menu.file")]
#[derive(Default)]
pub struct FileMenuRoot;

impl IContribution for FileMenuRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.menu.file")
    }
}

#[contribute(host_id = "studio.shell", id = "menu.view", kind = "menu", order = 2, label = "studio.menu.view")]
#[derive(Default)]
pub struct ViewMenuRoot;

impl IContribution for ViewMenuRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.menu.view")
    }
}

#[contribute(host_id = "studio.shell", id = "menu.help", kind = "menu", order = 3, label = "studio.menu.help")]
#[derive(Default)]
pub struct HelpMenuRoot;

impl IContribution for HelpMenuRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.menu.help")
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  叶子命令（ICommand 实现）
// ──────────────────────────────────────────────────────────────────────────

#[contribute(
    host_id = "studio.shell",
    id = "menu.file.exit",
    parent_id = "menu.file",
    command,
    kind = "menu",
    order = 1,
    label = "studio.menu.file_exit"
)]
#[derive(Default)]
pub struct ExitCommand;

impl IContribution for ExitCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.menu.file_exit")
    }
}

impl ICommand for ExitCommand {
    fn execute(&self, ctx: &mut CallContext) {
        ctx.app.quit();
    }
}

#[contribute(
    host_id = "studio.shell",
    id = "menu.view.theme",
    parent_id = "menu.view",
    command,
    kind = "menu",
    order = 1,
    label = "studio.menu.view_theme"
)]
#[derive(Default)]
pub struct ToggleThemeCommand;

impl IContribution for ToggleThemeCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.menu.view_theme")
    }
}

impl ICommand for ToggleThemeCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.apply_toggle_theme(cx);
        });
    }
}

#[contribute(
    host_id = "studio.shell",
    id = "menu.view.lang_en",
    parent_id = "menu.view",
    command,
    kind = "menu",
    order = 2,
    label = "studio.menu.view_lang_en"
)]
#[derive(Default)]
pub struct SwitchEnCommand;

impl IContribution for SwitchEnCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.menu.view_lang_en")
    }
}

impl ICommand for SwitchEnCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.apply_switch_en(cx);
        });
    }
}

#[contribute(
    host_id = "studio.shell",
    id = "menu.view.lang_zh",
    parent_id = "menu.view",
    command,
    kind = "menu",
    order = 3,
    label = "studio.menu.view_lang_zh"
)]
#[derive(Default)]
pub struct SwitchZhCommand;

impl IContribution for SwitchZhCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.menu.view_lang_zh")
    }
}

impl ICommand for SwitchZhCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.apply_switch_zh(cx);
        });
    }
}

#[contribute(
    host_id = "studio.shell",
    id = "menu.help.about",
    parent_id = "menu.help",
    command,
    kind = "menu",
    order = 1,
    label = "studio.menu.help_about"
)]
#[derive(Default)]
pub struct AboutCommand;

impl IContribution for AboutCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.menu.help_about")
    }
}

impl ICommand for AboutCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_welcome(cx);
        });
    }
}

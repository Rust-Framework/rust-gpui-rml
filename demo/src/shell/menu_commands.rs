//! 菜单命令贡献 —— 7 个叶子命令 + 5 个 submenu root，全部经 `#[contribute]` 注册到 `demo.shell`。
//!
//! 叶子命令实现 `IContribution` + `ICommand`，经 `as_command()` 查询；
//! submenu root 仅实现 `IContribution`（无命令），作为分组节点。
//! `MenuViewModel::from_contribution` 按 `kind="menu"` 过滤，按 `parent_id` 组织树。

use gpui::SharedString;
use rml::prelude::*;
use rml_core::command::{CallContext, ICommand};
use rml_core::i18n::t_static;

use crate::shell::main_window::{MainWindow, MainWindowRef};

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

#[contribute(host_id = "demo.shell", id = "menu.file", kind = "menu", order = 1, label = "menu.file")]
#[derive(Default)]
pub struct FileMenuRoot;

impl IContribution for FileMenuRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.file")
    }
}

#[contribute(host_id = "demo.shell", id = "menu.view", kind = "menu", order = 2, label = "menu.view")]
#[derive(Default)]
pub struct ViewMenuRoot;

impl IContribution for ViewMenuRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.view")
    }
}

#[contribute(host_id = "demo.shell", id = "menu.help", kind = "menu", order = 3, label = "menu.help")]
#[derive(Default)]
pub struct HelpMenuRoot;

impl IContribution for HelpMenuRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.help")
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.center",
    parent_id = "menu.help",
    kind = "menu",
    order = 1,
    label = "case.menu.help_center"
)]
#[derive(Default)]
pub struct HelpCenterMenuRoot;

impl IContribution for HelpCenterMenuRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.help_center")
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.features.group",
    parent_id = "menu.help",
    kind = "menu",
    order = 2,
    label = "case.menu.features.group"
)]
#[derive(Default)]
pub struct FeaturesGroupMenuRoot;

impl IContribution for FeaturesGroupMenuRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.features.group")
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  叶子命令（ICommand 实现）
// ──────────────────────────────────────────────────────────────────────────

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.new",
    parent_id = "menu.file",
    command,
    kind = "menu",
    order = 1,
    label = "menu.file_new"
)]
#[derive(Default)]
pub struct OpenWelcomeCommand;

impl IContribution for OpenWelcomeCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.file_new")
    }
}

impl ICommand for OpenWelcomeCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_case("welcome".to_string(), cx);
        });
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.open",
    parent_id = "menu.file",
    command,
    kind = "menu",
    order = 2,
    label = "menu.file_open"
)]
#[derive(Default)]
pub struct OpenButtonCaseCommand;

impl IContribution for OpenButtonCaseCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.file_open")
    }
}

impl ICommand for OpenButtonCaseCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_case("components.button".to_string(), cx);
        });
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.exit",
    parent_id = "menu.file",
    command,
    kind = "menu",
    order = 3,
    label = "menu.file_exit"
)]
#[derive(Default)]
pub struct ExitCommand;

impl IContribution for ExitCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.file_exit")
    }
}

impl ICommand for ExitCommand {
    fn execute(&self, ctx: &mut CallContext) {
        ctx.app.quit();
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.view.theme",
    parent_id = "menu.view",
    command,
    kind = "menu",
    order = 1,
    label = "menu.theme_toggle"
)]
#[derive(Default)]
pub struct ToggleThemeCommand;

impl IContribution for ToggleThemeCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.theme_toggle")
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
    host_id = "demo.shell",
    id = "menu.view.lang",
    parent_id = "menu.view",
    command,
    kind = "menu",
    order = 2,
    label = "menu.lang_en"
)]
#[derive(Default)]
pub struct SwitchEnCommand;

impl IContribution for SwitchEnCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.lang_en")
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
    host_id = "demo.shell",
    id = "menu.help.nested",
    parent_id = "menu.help.center",
    command,
    kind = "menu",
    order = 1,
    label = "case.menu.nested"
)]
#[derive(Default)]
pub struct OpenMenuDropdownCaseCommand;

impl IContribution for OpenMenuDropdownCaseCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.nested")
    }
}

impl ICommand for OpenMenuDropdownCaseCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_case("components.menu.dropdown".to_string(), cx);
        });
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.about",
    parent_id = "menu.help.center",
    command,
    kind = "menu",
    order = 2,
    label = "menu.help_about"
)]
#[derive(Default)]
pub struct OpenAboutCommand;

impl IContribution for OpenAboutCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.help_about")
    }
}

impl ICommand for OpenAboutCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_case("welcome".to_string(), cx);
        });
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.features",
    parent_id = "menu.help.features.group",
    command,
    kind = "menu",
    order = 1,
    label = "case.menu.features.title"
)]
#[derive(Default)]
pub struct OpenFeaturesCaseCommand;

impl IContribution for OpenFeaturesCaseCommand {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.features.title")
    }
}

impl ICommand for OpenFeaturesCaseCommand {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_case("components.menu.features".to_string(), cx);
        });
    }
}

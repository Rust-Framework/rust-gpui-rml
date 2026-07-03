//! Shell 菜单贡献：手写 `impl IContribution`（submenu root）+ `impl ICommand`（leaf）。
//!
//! 叶子项通过 `DemoShellHost` 全局获取 `WeakEntity<MainWindow>`，在 `execute` 中
//! upgrade + update 调用 MainWindow 方法（与 `ActivityPanel::on_case_activate` 模式一致）。

use gpui::SharedString;
use rml::prelude::*;
use rml_core::command::{CallContext, ICommand};
use rml_core::i18n::t_static;

use crate::shell::{DemoShellHost, MainWindow};

/// 命令执行 helper：从 `DemoShellHost` 全局获取 `MainWindow` 弱引用，
/// upgrade 后在闭包中执行 MainWindow 方法。统一 6 处 `try_global`+`upgrade`+`update` 样板。
fn with_main_window<F>(ctx: &mut CallContext, f: F)
where
    F: FnOnce(&mut MainWindow, &mut gpui::Context<MainWindow>),
{
    if let Some(host) = ctx
        .app
        .try_global::<DemoShellHost>()
        .and_then(|h| h.0.upgrade())
    {
        host.update(ctx.app, |this, cx| f(this, cx));
    }
}

// ── File（二级） ──

#[contribute(host_id = "demo.shell", id = "menu.file", kind = "menu", order = 0)]
#[derive(Default)]
pub struct MenuFileRoot;

impl IContribution for MenuFileRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.file").into()
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.new",
    parent_id = "menu.file",
    order = 0,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuFileNew;

impl IContribution for MenuFileNew {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.file_new").into()
    }
}

impl ICommand for MenuFileNew {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| this.open_case("welcome".to_string(), cx));
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.open",
    parent_id = "menu.file",
    order = 1,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuFileOpen;

impl IContribution for MenuFileOpen {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.file_open").into()
    }
}

impl ICommand for MenuFileOpen {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_case("components.button".to_string(), cx)
        });
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.exit",
    parent_id = "menu.file",
    order = 2,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuFileExit;

impl IContribution for MenuFileExit {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.file_exit").into()
    }
}

impl ICommand for MenuFileExit {
    fn execute(&self, ctx: &mut CallContext) {
        ctx.app.quit();
    }
}

// ── View（二级） ──

#[contribute(host_id = "demo.shell", id = "menu.view", kind = "menu", order = 10)]
#[derive(Default)]
pub struct MenuViewRoot;

impl IContribution for MenuViewRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.view").into()
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.theme_toggle",
    parent_id = "menu.view",
    order = 0,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuThemeToggleContrib;

impl IContribution for MenuThemeToggleContrib {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.theme_toggle").into()
    }
}

impl ICommand for MenuThemeToggleContrib {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| this.apply_toggle_theme(cx));
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.lang_en",
    parent_id = "menu.view",
    order = 1,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuLangEnContrib;

impl IContribution for MenuLangEnContrib {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.lang_en").into()
    }
}

impl ICommand for MenuLangEnContrib {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| this.apply_switch_en(cx));
    }
}

// ── Help（三级：Help → Docs → Guide/About） ──

#[contribute(host_id = "demo.shell", id = "menu.help", kind = "menu", order = 20)]
#[derive(Default)]
pub struct MenuHelpRoot;

impl IContribution for MenuHelpRoot {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.help").into()
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.docs",
    parent_id = "menu.help",
    order = 0,
    kind = "menu",
)]
#[derive(Default)]
pub struct MenuHelpDocs;

impl IContribution for MenuHelpDocs {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.help_center").into()
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.guide",
    parent_id = "menu.help.docs",
    order = 0,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuHelpGuide;

impl IContribution for MenuHelpGuide {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.nested").into()
    }
}

impl ICommand for MenuHelpGuide {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_case("components.menu.dropdown".to_string(), cx)
        });
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.about",
    parent_id = "menu.help.docs",
    order = 1,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuHelpAbout;

impl IContribution for MenuHelpAbout {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("menu.help_about").into()
    }
}

impl ICommand for MenuHelpAbout {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| this.open_case("welcome".to_string(), cx));
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.cases",
    parent_id = "menu.help",
    order = 1,
    kind = "menu",
)]
#[derive(Default)]
pub struct MenuHelpCases;

impl IContribution for MenuHelpCases {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.features.group").into()
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "menu.open_features",
    parent_id = "menu.help.cases",
    order = 0,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuOpenFeaturesContrib;

impl IContribution for MenuOpenFeaturesContrib {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.features.title").into()
    }
}

impl ICommand for MenuOpenFeaturesContrib {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| {
            this.open_case("components.menu.features".to_string(), cx)
        });
    }
}

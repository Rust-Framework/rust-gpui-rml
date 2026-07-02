//! Shell 菜单贡献：二/三级子菜单演示（叶子命令在 `MainWindow.menu_commands` 绑定）

use rml::prelude::*;

// ── File（二级） ──

#[contribute(host_id = "demo.shell", id = "menu.file", name = "menu.file", kind = "menu", order = 0)]
#[derive(Default)]
pub struct MenuFileRoot;

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.new",
    name = "menu.file_new",
    kind = "menu",
    parent_id = "menu.file",
    order = 0,
)]
#[derive(Default)]
pub struct MenuFileNew;

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.open",
    name = "menu.file_open",
    kind = "menu",
    parent_id = "menu.file",
    order = 1,
)]
#[derive(Default)]
pub struct MenuFileOpen;

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.exit",
    name = "menu.file_exit",
    kind = "menu",
    parent_id = "menu.file",
    order = 2,
)]
#[derive(Default)]
pub struct MenuFileExit;

// ── View（二级） ──

#[contribute(host_id = "demo.shell", id = "menu.view", name = "menu.view", kind = "menu", order = 10)]
#[derive(Default)]
pub struct MenuViewRoot;

#[contribute(
    host_id = "demo.shell",
    id = "menu.theme_toggle",
    name = "menu.theme_toggle",
    kind = "menu",
    parent_id = "menu.view",
    order = 0,
)]
#[derive(Default)]
pub struct MenuThemeToggleContrib;

#[contribute(
    host_id = "demo.shell",
    id = "menu.lang_en",
    name = "menu.lang_en",
    kind = "menu",
    parent_id = "menu.view",
    order = 1,
)]
#[derive(Default)]
pub struct MenuLangEnContrib;

// ── Help（三级：Help → Docs → Guide/About） ──

#[contribute(host_id = "demo.shell", id = "menu.help", name = "menu.help", kind = "menu", order = 20)]
#[derive(Default)]
pub struct MenuHelpRoot;

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.docs",
    name = "case.menu.help_center",
    kind = "menu",
    parent_id = "menu.help",
    order = 0,
)]
#[derive(Default)]
pub struct MenuHelpDocs;

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.guide",
    name = "case.menu.nested",
    kind = "menu",
    parent_id = "menu.help.docs",
    order = 0,
)]
#[derive(Default)]
pub struct MenuHelpGuide;

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.about",
    name = "menu.help_about",
    kind = "menu",
    parent_id = "menu.help.docs",
    order = 1,
)]
#[derive(Default)]
pub struct MenuHelpAbout;

#[contribute(
    host_id = "demo.shell",
    id = "menu.help.cases",
    name = "case.menu.features.group",
    kind = "menu",
    parent_id = "menu.help",
    order = 1,
)]
#[derive(Default)]
pub struct MenuHelpCases;

#[contribute(
    host_id = "demo.shell",
    id = "menu.open_features",
    name = "case.menu.features.title",
    kind = "menu",
    parent_id = "menu.help.cases",
    order = 0,
)]
#[derive(Default)]
pub struct MenuOpenFeaturesContrib;

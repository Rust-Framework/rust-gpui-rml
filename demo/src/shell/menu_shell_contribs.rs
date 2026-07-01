//! Shell 菜单贡献：二/三级子菜单演示（叶子命令在 `MainWindow.menu_commands` 绑定）

use rml::prelude::*;

use super::MainWindow;

// ── File（二级） ──

#[contribute(host = MainWindow, id = "menu.file", name = "menu.file", kind = "menu", order = 0)]
#[derive(Default)]
pub struct MenuFileRoot;

#[contribute(
    host = MainWindow,
    id = "menu.file.new",
    name = "menu.file_new",
    kind = "menu",
    parent_id = "menu.file",
    order = 0,
)]
#[derive(Default)]
pub struct MenuFileNew;

#[contribute(
    host = MainWindow,
    id = "menu.file.open",
    name = "menu.file_open",
    kind = "menu",
    parent_id = "menu.file",
    order = 1,
)]
#[derive(Default)]
pub struct MenuFileOpen;

#[contribute(
    host = MainWindow,
    id = "menu.file.exit",
    name = "menu.file_exit",
    kind = "menu",
    parent_id = "menu.file",
    order = 2,
)]
#[derive(Default)]
pub struct MenuFileExit;

// ── View（二级） ──

#[contribute(host = MainWindow, id = "menu.view", name = "menu.view", kind = "menu", order = 10)]
#[derive(Default)]
pub struct MenuViewRoot;

#[contribute(
    host = MainWindow,
    id = "menu.theme_toggle",
    name = "menu.theme_toggle",
    kind = "menu",
    parent_id = "menu.view",
    order = 0,
)]
#[derive(Default)]
pub struct MenuThemeToggleContrib;

#[contribute(
    host = MainWindow,
    id = "menu.lang_en",
    name = "menu.lang_en",
    kind = "menu",
    parent_id = "menu.view",
    order = 1,
)]
#[derive(Default)]
pub struct MenuLangEnContrib;

// ── Help（三级：Help → Docs → Guide/About） ──

#[contribute(host = MainWindow, id = "menu.help", name = "menu.help", kind = "menu", order = 20)]
#[derive(Default)]
pub struct MenuHelpRoot;

#[contribute(
    host = MainWindow,
    id = "menu.help.docs",
    name = "case.menu.help_center",
    kind = "menu",
    parent_id = "menu.help",
    order = 0,
)]
#[derive(Default)]
pub struct MenuHelpDocs;

#[contribute(
    host = MainWindow,
    id = "menu.help.guide",
    name = "case.menu.nested",
    kind = "menu",
    parent_id = "menu.help.docs",
    order = 0,
)]
#[derive(Default)]
pub struct MenuHelpGuide;

#[contribute(
    host = MainWindow,
    id = "menu.help.about",
    name = "menu.help_about",
    kind = "menu",
    parent_id = "menu.help.docs",
    order = 1,
)]
#[derive(Default)]
pub struct MenuHelpAbout;

#[contribute(
    host = MainWindow,
    id = "menu.help.cases",
    name = "case.menu.features.group",
    kind = "menu",
    parent_id = "menu.help",
    order = 1,
)]
#[derive(Default)]
pub struct MenuHelpCases;

#[contribute(
    host = MainWindow,
    id = "menu.open_features",
    name = "case.menu.features.title",
    kind = "menu",
    parent_id = "menu.help.cases",
    order = 0,
)]
#[derive(Default)]
pub struct MenuOpenFeaturesContrib;

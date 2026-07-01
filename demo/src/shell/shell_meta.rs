//! Shell metadata contributions (categories, menu/status entries) ? auto-registered by build.rs

use super::MainWindow;
use rml::contribute;

#[contribute(host = MainWindow, id = "cat.binding", name = "tree.cat.binding", kind = "case", order = 0)]
#[derive(Default)]
pub struct CatBinding;

#[contribute(host = MainWindow, id = "cat.components", name = "tree.cat.components", kind = "case", order = 10)]
#[derive(Default)]
pub struct CatComponents;

#[contribute(host = MainWindow, id = "cat.menu", name = "tree.cat.menu", kind = "case", order = 15)]
#[derive(Default)]
pub struct CatMenu;

#[contribute(host = MainWindow, id = "cat.i18n", name = "tree.cat.i18n", kind = "case", order = 20)]
#[derive(Default)]
pub struct CatI18n;

#[contribute(host = MainWindow, id = "menu.theme_toggle", name = "menu.theme_toggle", kind = "menu", order = 0)]
#[derive(Default)]
pub struct MenuThemeToggle;

#[contribute(host = MainWindow, id = "menu.lang_en", name = "menu.lang_en", kind = "menu", order = 10)]
#[derive(Default)]
pub struct MenuLangEn;

#[contribute(host = MainWindow, id = "status.ready", name = "shell.status_ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

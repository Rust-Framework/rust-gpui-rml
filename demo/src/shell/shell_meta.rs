//! Shell metadata：案例树分类 + 状态栏（菜单项由各 case 模块自行 `#[contribute]` 注册）

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

#[contribute(host = MainWindow, id = "status.ready", name = "shell.status_ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

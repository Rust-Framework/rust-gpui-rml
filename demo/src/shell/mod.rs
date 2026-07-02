pub mod menu_shell_contribs;
pub mod shell_chrome;
pub mod shell_meta;
#[path = "activity_panel.rml.rs"]
pub mod activity_panel;
#[path = "login_dialog.rml.rs"]
pub mod login_dialog;
#[path = "main_window.rml.rs"]
pub mod main_window;

pub use main_window::{DemoShellHost, MainWindow};

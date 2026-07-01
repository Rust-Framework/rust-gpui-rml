pub mod menu_shell_contribs;
pub mod shell_chrome;
pub mod shell_meta;
#[path = "case_activity_panel.rml.rs"]
pub mod case_activity_panel;
#[path = "case_host.rml.rs"]
pub mod case_host;
#[path = "login_dialog.rml.rs"]
pub mod login_dialog;
#[path = "main_window.rml.rs"]
pub mod main_window;

pub use main_window::MainWindow;

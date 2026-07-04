pub mod case_view_model;
pub mod menu_commands;
pub mod menu_view_model;
pub mod status_view_model;
pub mod workbench;
#[path = "activity_panel.rml.rs"]
pub mod activity_panel;
#[path = "login_dialog.rml.rs"]
pub mod login_dialog;
#[path = "main_window.rml.rs"]
pub mod main_window;

pub use main_window::{MainWindow, MainWindowRef};

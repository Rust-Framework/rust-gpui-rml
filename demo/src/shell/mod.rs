pub mod case_view_model;
pub mod menu_commands;
pub mod menu_view_model;
pub mod status_view_model;
pub mod workbench;
pub mod bottom_tabs;
#[path = "activity_panel.rml.rs"]
pub mod activity_panel;
#[path = "activity_act.rml.rs"]
pub mod activity_act;
#[path = "login_dialog.rml.rs"]
pub mod login_dialog;
#[path = "settings_dialog.rml.rs"]
pub mod settings_dialog;
#[path = "main_window.rml.rs"]
pub mod main_window;

pub use main_window::{MainWindow, MainWindowRef};

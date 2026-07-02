//! RML 案例库模块

pub mod catalog;
#[path = "welcome_case.rml.rs"]
pub mod welcome_case;
#[path = "counter_case.rml.rs"]
pub mod counter_case;
#[path = "two_way_case.rml.rs"]
pub mod two_way_case;
#[path = "button_case.rml.rs"]
pub mod button_case;
#[path = "i18n_case.rml.rs"]
pub mod i18n_case;
#[path = "menu_context_case.rml.rs"]
pub mod menu_context_case;
#[path = "menu_dropdown_case.rml.rs"]
pub mod menu_dropdown_case;
#[path = "menu_editor_case.rml.rs"]
pub mod menu_editor_case;
#[path = "menu_features_case.rml.rs"]
pub mod menu_features_case;
#[path = "menu_custom_case.rml.rs"]
pub mod menu_custom_case;
#[path = "status_bar_case.rml.rs"]
pub mod status_bar_case;

pub use catalog::{case_title_key, OpenTab};

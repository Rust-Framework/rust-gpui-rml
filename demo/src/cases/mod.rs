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

pub use catalog::{case_title_key, init_tree_state, refresh_tree_state, OpenTab};
pub use button_case::ButtonCase;
pub use counter_case::CounterCase;
pub use i18n_case::I18nCase;
pub use two_way_case::TwoWayCase;
pub use welcome_case::WelcomeCase;

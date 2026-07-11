use gpui::SharedString;
pub use gpui_component::combobox::{Combobox, ComboboxEvent, ComboboxState};
pub use gpui_component::searchable_list::SearchableVec;

pub type StringComboboxState = ComboboxState<SearchableVec<SharedString>>;
pub type StringComboboxEvent = ComboboxEvent<SearchableVec<SharedString>>;

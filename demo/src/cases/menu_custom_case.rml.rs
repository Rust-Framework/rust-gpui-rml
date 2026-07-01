use rml::prelude::*;

#[contribute(
    host = "demo.shell",
    id = "components.menu.custom",
    name = "case.menu.custom.title",
    kind = "case",
    parent_id = "cat.menu",
    order = 20,
)]
#[component]
#[derive(Default)]
pub struct MenuCustomCase {
    pub dark_mode: bool,
    pub last_action: String,
}

impl ILifecycle for MenuCustomCase {}

impl MenuCustomCase {
    #[computed]
    pub fn dark_mode_label(&self) -> String {
        if self.dark_mode {
            rml_core::i18n::t_static("case.menu.on").to_string()
        } else {
            rml_core::i18n::t_static("case.menu.off").to_string()
        }
    }

    #[computed]
    pub fn custom_status(&self) -> String {
        self.last_action.clone()
    }

    #[command]
    pub fn on_toggle_dark(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.dark_mode = !self.dark_mode;
        self.last_action = format!("Dark mode: {}", self.dark_mode);
    }

    #[command]
    pub fn on_sign_out(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Sign Out".to_string();
    }
}

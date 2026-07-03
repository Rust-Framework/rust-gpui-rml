use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.menu.dropdown",
    kind = "case",
    group = "menu",
    order = 17,
)]
#[component]
#[derive(Default)]
pub struct MenuDropdownCase {
    pub last_action: String,
}

impl IContribution for MenuDropdownCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.dropdown.title").into()
    }
}

impl ILifecycle for MenuDropdownCase {}

impl MenuDropdownCase {
    #[computed]
    pub fn dropdown_status(&self) -> String {
        if self.last_action.is_empty() {
            rml_core::i18n::t_static("case.menu.dropdown.idle").to_string()
        } else {
            self.last_action.clone()
        }
    }

    #[command]
    pub fn on_custom(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Custom Action".to_string();
    }

    #[command]
    pub fn on_standard(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Standard Action".to_string();
    }

    #[command]
    pub fn on_exit(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Exit".to_string();
    }
}

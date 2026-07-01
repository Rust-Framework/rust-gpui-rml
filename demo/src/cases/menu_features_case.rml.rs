use rml::prelude::*;
use crate::shell::MainWindow;

#[contribute(
    host = MainWindow,
    id = "components.menu.features",
    name = "case.menu.features.title",
    kind = "case",
    parent_id = "cat.menu",
    order = 19,
)]
#[component]
#[derive(Default)]
pub struct MenuFeaturesCase {
    pub is_checked: bool,
    pub last_action: String,
}

impl ILifecycle for MenuFeaturesCase {}

impl MenuFeaturesCase {
    #[computed]
    pub fn features_status(&self) -> String {
        self.last_action.clone()
    }

    #[command]
    pub fn on_available(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Available".to_string();
    }

    #[command]
    pub fn on_disabled(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Disabled (should not fire)".to_string();
    }

    #[command]
    pub fn on_toggle_check(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.is_checked = !self.is_checked;
        self.last_action = format!("Checked: {}", self.is_checked);
    }

    #[command]
    pub fn on_nested_a(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Nested A".to_string();
    }

    #[command]
    pub fn on_nested_b(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Nested B".to_string();
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.menu.features",
    kind = "case",
    group = "menu",
    order = 19,
)]
#[component]
#[derive(Default)]
pub struct MenuFeaturesCase {
    pub is_checked: bool,
    pub last_action: String,
}

impl IContribution for MenuFeaturesCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.features.title").into()
    }
}

impl ILifecycle for MenuFeaturesCase {}

impl MenuFeaturesCase {
    #[computed]
    pub fn features_status(&self) -> String {
        self.last_action.clone()
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<dropdown-menu scrollable="" max_h="280">
    <Button label="Features" ghost="" />
    <menu-item label="Available" onclick={on_available} />
    <menu-item label="Disabled" disabled="" onclick={on_disabled} />
    <menu-item label="Checkable" checked={is_checked} onclick={on_toggle_check} />
    <menu-separator />
    <menu-item label="Docs" href="https://..." icon="Info" />
    <menu-item label="Submenu">
        <menu-item label="Item A" onclick={on_nested_a} />
    </menu-item>
</dropdown-menu>"#
            .to_string()
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

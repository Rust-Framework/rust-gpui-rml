use rml::prelude::*;

#[contribute(
    host_id = "demo.shell",
    id = "components.menu.context",
    name = "case.menu.context.title",
    kind = "case",
    parent_id = "cat.menu",
    order = 16,
)]
#[component]
#[derive(Default)]
pub struct MenuContextCase {
    pub last_action: String,
}

impl ILifecycle for MenuContextCase {}

impl MenuContextCase {
    #[computed]
    pub fn context_status(&self) -> String {
        if self.last_action.is_empty() {
            rml_core::i18n::t_static("case.menu.context.idle").to_string()
        } else {
            format!(
                "{}: {}",
                rml_core::i18n::t_static("case.menu.last_action"),
                self.last_action
            )
        }
    }

    fn set_action(&mut self, name: &str) {
        self.last_action = name.to_string();
    }

    #[command]
    pub fn on_open(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Open");
    }

    #[command]
    pub fn on_copy(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Copy");
    }

    #[command]
    pub fn on_cut(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Cut");
    }

    #[command]
    pub fn on_paste(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Paste");
    }

    #[command]
    pub fn on_new_file(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("New File");
    }

    #[command]
    pub fn on_new_folder(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("New Folder");
    }

    #[command]
    pub fn on_delete(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Delete");
    }
}

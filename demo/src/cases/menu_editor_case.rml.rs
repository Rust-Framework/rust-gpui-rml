use rml::prelude::*;

#[contribute(
    host_id = "demo.shell",
    id = "components.menu.editor",
    name = "case.menu.editor.title",
    kind = "case",
    group = "menu",
    order = 18,
)]
#[component]
#[derive(Default)]
pub struct MenuEditorCase {
    pub word_wrap: bool,
    pub last_action: String,
}

impl ILifecycle for MenuEditorCase {}

impl MenuEditorCase {
    #[computed]
    pub fn editor_status(&self) -> String {
        if self.last_action.is_empty() {
            format!(
                "{}: {}",
                rml_core::i18n::t_static("case.menu.word_wrap"),
                if self.word_wrap { "on" } else { "off" }
            )
        } else {
            self.last_action.clone()
        }
    }

    #[command]
    pub fn on_save(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Save".to_string();
    }

    #[command]
    pub fn on_save_as(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Save As".to_string();
    }

    #[command]
    pub fn on_find(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Find".to_string();
    }

    #[command]
    pub fn on_replace(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Replace".to_string();
    }

    #[command]
    pub fn on_toggle_wrap(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.word_wrap = !self.word_wrap;
        self.last_action = format!("Word Wrap: {}", self.word_wrap);
    }
}

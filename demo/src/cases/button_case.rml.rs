use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.button",
    kind = "case",
    group = "components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub basic_clicks: i32,
    pub is_disabled: bool,
    pub is_selected: bool,
}

impl IContribution for ButtonCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.button.title")
    }
}

impl ButtonCase {
    #[computed]
    pub fn basic_click_text(&self) -> String {
        format!("点击次数：{}", self.basic_clicks)
    }

    #[computed]
    pub fn disabled_status_text(&self) -> String {
        if self.is_disabled {
            "当前：禁用".to_string()
        } else {
            "当前：可用".to_string()
        }
    }

    #[computed]
    pub fn selected_status_text(&self) -> String {
        if self.is_selected {
            "当前：选中".to_string()
        } else {
            "当前：未选中".to_string()
        }
    }

    #[command]
    pub fn on_basic_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.basic_clicks += 1;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_toggle_selected(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_selected = !self.is_selected;
    }
}

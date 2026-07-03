use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.button",
    kind = "case",
    group = "components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub button_clicks: i32,
    pub is_disabled: bool,
    pub is_selected: bool,
}

impl IContribution for ButtonCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.button.title").into()
    }
}

impl ButtonCase {
    #[computed]
    pub fn button_demo_text(&self) -> String {
        format!("按钮点击：{}", self.button_clicks)
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Button label="提交" primary="" onclick={on_submit} />
<Button label={t("demo.click_btn")} ghost="" onclick={on_click} />
<Button label="禁用" disabled={is_disabled} />"#.to_string()
    }

    #[command]
    pub fn on_button_demo_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.button_clicks += 1;
    }

    #[command]
    pub fn on_toggle_disabled_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_toggle_selected_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_selected = !self.is_selected;
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.checkbox",
    kind = "case",
    group = "components",
    order = 33,
)]
#[component]
#[derive(Default)]
pub struct CheckboxCase {
    pub is_checked: bool,
    pub is_disabled: bool,
}

impl IContribution for CheckboxCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.checkbox.title")
    }
}

impl CheckboxCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.is_checked {
            "当前：已勾选".to_string()
        } else {
            "当前：未勾选".to_string()
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Checkbox label="同意条款" checked={is_checked} />
<Checkbox label="禁用项" checked={is_disabled} disabled={is_disabled} />"#
            .to_string()
    }

    #[command]
    pub fn on_toggle_checked(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_checked = !self.is_checked;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
        cx.notify();
    }
}

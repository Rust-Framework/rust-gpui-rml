use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.switch",
    kind = "case",
    group = "components",
    order = 34,
)]
#[component]
#[derive(Default)]
pub struct SwitchCase {
    pub is_on: bool,
    pub is_disabled: bool,
}

impl IContribution for SwitchCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.switch.title")
    }
}

impl SwitchCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.is_on {
            "当前：开启".to_string()
        } else {
            "当前：关闭".to_string()
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Switch label="自动保存" checked={is_on} />
<Switch checked={is_disabled} disabled={is_disabled} />"#
            .to_string()
    }

    #[command]
    pub fn on_toggle(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_on = !self.is_on;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
        cx.notify();
    }
}

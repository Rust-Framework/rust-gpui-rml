use gpui::SharedString;
use rml::prelude::*;

use crate::cases::common::CaseDocPage;

#[contribute(
    host_id = "demo.shell",
    id = "framework.css-functions",
    kind = "case",
    group = "framework",
    order = 48,
)]
#[component]
#[derive(Default)]
pub struct CssFunctionsCase {
    pub is_dark: bool,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for CssFunctionsCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        "颜色函数与单位".into()
    }
}

impl ILifecycle for CssFunctionsCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.is_dark = false;
    }
}

impl CssFunctionsCase {
    #[computed]
    pub fn theme_label(&self) -> &'static str {
        if self.is_dark {
            "深色（dark）"
        } else {
            "浅色（light）"
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("css_functions_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("css_functions_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_toggle_theme(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_dark = !self.is_dark;
    }
}

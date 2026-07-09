use gpui::SharedString;
use rml::prelude::*;

use crate::cases::common::CaseDocPage;

#[contribute(
    host_id = "demo.shell",
    id = "framework.css-priority",
    kind = "case",
    group = "framework",
    order = 47,
)]
#[component]
#[derive(Default)]
pub struct CssPriorityCase {
    pub is_active: bool,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for CssPriorityCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        "CSS 优先级".into()
    }
}

impl ILifecycle for CssPriorityCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.is_active = true;
    }
}

impl CssPriorityCase {
    #[computed]
    pub fn status_label(&self) -> &'static str {
        if self.is_active {
            "激活（绿色）"
        } else {
            "停用（红色）"
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("css_priority_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("css_priority_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_activate(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_active = true;
    }

    #[command]
    pub fn on_deactivate(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_active = false;
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage, EventButton};

#[contribute(
    host_id = "demo.shell",
    id = "components.user_event",
    kind = "case",
    group = "components",
    order = 90,
)]
#[component]
#[derive(Default)]
pub struct UserComponentEventCase {
    pub click_count: i32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub event_button: Option<gpui::Entity<EventButton>>,
}

impl IContribution for UserComponentEventCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.user_event.title")
    }
}

impl ILifecycle for UserComponentEventCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.event_button = Some(cx.new(|_cx| EventButton::default()));
        let (cols, rows) = build_api_table(&[
            ("label", "string / binding", "按钮文字"),
            ("on-click", "event", "点击时回调"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl UserComponentEventCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("user_component_event_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("user_component_event_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_button_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.click_count += 1;
        cx.notify();
    }
}

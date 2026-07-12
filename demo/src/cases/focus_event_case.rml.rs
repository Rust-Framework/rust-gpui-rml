use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.focus_event",
    kind = "case",
    group = "framework",
    order = 52,
)]
#[component]
#[derive(Default)]
pub struct FocusEventCase {
    pub focus_count: u32,
    pub blur_count: u32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for FocusEventCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.focus_event.title")
    }
}

impl ILifecycle for FocusEventCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("on-focus", "event", "获得焦点时回调"),
            ("on-blur", "event", "失去焦点时回调"),
            ("focusable", "bool", "使元素可接收焦点，如 focusable=\"\" 或 focusable=\"true\""),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl FocusEventCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("focus_event_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("focus_event_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_focus(&mut self, _: &FocusEvent, _cx: &mut Context<Self>) {
        self.focus_count = self.focus_count.saturating_add(1);
    }

    #[command]
    pub fn on_blur(&mut self, _: &FocusEvent, _cx: &mut Context<Self>) {
        self.blur_count = self.blur_count.saturating_add(1);
    }
}

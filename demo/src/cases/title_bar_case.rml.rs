use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.title_bar",
    kind = "case",
    group = "components",
    order = 31,
)]
#[component]
#[derive(Default)]
pub struct TitleBarCase {
    pub title: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TitleBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.title_bar.title")
    }
}

impl ILifecycle for TitleBarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.title = "RML Showcase".into();
        let (cols, rows) = build_api_table(&[
            ("子节点", "元素[]", "中央区域内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TitleBarCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("title_bar_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("title_bar_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_reset_title(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.title = "RML Showcase".into();
    }
}

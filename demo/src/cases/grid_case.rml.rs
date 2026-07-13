use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.grid",
    kind = "case",
    group = "layout",
    order = 58,
)]
#[component]
#[derive(Default)]
pub struct GridCase {
    pub field1_input: ElementRef<rml_ui::InputState>,
    pub field2_input: ElementRef<rml_ui::InputState>,
    pub remark_input: ElementRef<rml_ui::InputState>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for GridCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.grid.title")
    }
}

impl ILifecycle for GridCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("columns", "number", "等宽列数，如 columns=\"3\""),
            ("rows", "number", "等高行数，如 rows=\"2\""),
            ("col-span", "number", "GridItem 跨列数，如 col-span=\"2\""),
            ("row-span", "number", "GridItem 跨行数，如 row-span=\"2\""),
            ("col-start", "number", "GridItem 起始列（支持负数，从末尾计数）"),
            ("col-end", "number", "GridItem 结束列"),
            ("row-start", "number", "GridItem 起始行"),
            ("row-end", "number", "GridItem 结束行"),
            ("gap", "string", "网格间距，如 gap=\"12px\""),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl GridCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("grid_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("grid_case.rml.rs").to_string()
    }
}

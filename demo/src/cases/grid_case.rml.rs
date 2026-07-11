use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.grid",
    kind = "case",
    group = "framework",
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
            ("columns", "static: u16", "等宽列数，如 columns=\"3\" 创建 3 列等宽网格"),
            ("rows", "static: u16", "等高行数，如 rows=\"2\" 创建 2 行等高网格"),
            ("col-span", "static: u16", "GridItem 跨列数，如 col-span=\"2\" 跨 2 列"),
            ("row-span", "static: u16", "GridItem 跨行数，如 row-span=\"2\" 跨 2 行"),
            ("col-start", "static: i16", "GridItem 起始列位置（支持负数，从末尾计数）"),
            ("col-end", "static: i16", "GridItem 结束列位置"),
            ("row-start", "static: i16", "GridItem 起始行位置"),
            ("row-end", "static: i16", "GridItem 结束行位置"),
            ("gap", "style", "CSS gap 属性，控制网格间距，如 gap=\"12px\""),
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

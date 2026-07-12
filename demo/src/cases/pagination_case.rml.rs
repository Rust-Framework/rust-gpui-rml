use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.pagination",
    kind = "case",
    group = "components",
    order = 68,
)]
#[component]
#[derive(Default)]
pub struct PaginationCase {
    pub current_page: usize,
    pub total_pages: usize,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for PaginationCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.pagination.title")
    }
}

impl ILifecycle for PaginationCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.current_page = 1;
        self.total_pages = 10;
        let (cols, rows) = build_api_table(&[
            ("current-page", "number / binding", "当前页码（从 1 开始），如 current-page={page}"),
            ("total-pages", "number / binding", "总页数，如 total-pages={10}"),
            ("visible-pages", "number", "最大可见页码数（默认 5）"),
            ("compact", "bool", "紧凑模式（仅前后箭头）"),
            ("on-click", "event", "页码切换时回调，参数为页码"),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("disabled", "bool / binding", "禁用交互"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl PaginationCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("pagination_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("pagination_case.rml.rs").to_string()
    }

    // on_click 事件签名为 Fn(&usize, ...)（页码）
    #[command]
    pub fn on_page_change(&mut self, page: &usize, _cx: &mut Context<Self>) {
        self.current_page = *page;
    }
}

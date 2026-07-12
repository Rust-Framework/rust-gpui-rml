use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{BreadcrumbItem, BreadcrumbSibling, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.breadcrumb",
    kind = "case",
    group = "components",
    order = 92,
)]
#[component]
#[derive(Default)]
pub struct BreadcrumbCase {
    pub breadcrumb_items: Vec<BreadcrumbItem>,
    pub single_item: Vec<BreadcrumbItem>,
    pub path_items: Vec<BreadcrumbItem>,
    pub selected_level: usize,
    pub selected_index: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for BreadcrumbCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.breadcrumb.title")
    }
}

impl ILifecycle for BreadcrumbCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        self.breadcrumb_items = vec![
            BreadcrumbItem::new("项目")
                .siblings(vec![
                    BreadcrumbSibling::new("项目"),
                    BreadcrumbSibling::new("文档"),
                    BreadcrumbSibling::new("设置"),
                ]),
            BreadcrumbItem::new("src")
                .siblings(vec![
                    BreadcrumbSibling::new("src"),
                    BreadcrumbSibling::new("tests"),
                    BreadcrumbSibling::new("docs"),
                ]),
            BreadcrumbItem::new("main.rs")
                .siblings(vec![
                    BreadcrumbSibling::new("main.rs"),
                    BreadcrumbSibling::new("lib.rs"),
                    BreadcrumbSibling::new("mod.rs"),
                ]),
        ];

        self.single_item = vec![
            BreadcrumbItem::new("首页"),
        ];

        self.path_items = vec![
            BreadcrumbItem::new("home")
                .siblings(vec![
                    BreadcrumbSibling::new("home"),
                    BreadcrumbSibling::new("var"),
                    BreadcrumbSibling::new("etc"),
                ]),
            BreadcrumbItem::new("user")
                .siblings(vec![
                    BreadcrumbSibling::new("user"),
                    BreadcrumbSibling::new("admin"),
                    BreadcrumbSibling::new("guest"),
                ]),
            BreadcrumbItem::new("documents")
                .siblings(vec![
                    BreadcrumbSibling::new("documents"),
                    BreadcrumbSibling::new("downloads"),
                    BreadcrumbSibling::new("pictures"),
                ]),
            BreadcrumbItem::new("report.pdf")
                .siblings(vec![
                    BreadcrumbSibling::new("report.pdf"),
                    BreadcrumbSibling::new("summary.pdf"),
                    BreadcrumbSibling::new("data.csv"),
                ]),
        ];

        let (cols, rows) = build_api_table(&[
            ("items", "binding", "面包屑项列表，如 items={breadcrumb_items}"),
            ("on-select", "event", "同级选择时回调，参数为 (层级, 索引)"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl BreadcrumbCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("breadcrumb_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("breadcrumb_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_breadcrumb_select(&mut self, level: usize, index: usize, _cx: &mut Context<Self>) {
        self.selected_level = level;
        self.selected_index = index;
    }
}

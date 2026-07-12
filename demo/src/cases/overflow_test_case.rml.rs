use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

/// 框架能力案例 —— 验证 CSS overflow-x / overflow-y 映射与滚动容器行为。
#[contribute(
    host_id = "demo.shell",
    id = "framework.overflow",
    kind = "case",
    group = "framework",
    order = 47,
)]
#[component]
#[derive(Default)]
pub struct OverflowTestCase {
    pub items: Vec<SharedString>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for OverflowTestCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.overflow.title")
    }
}

impl ILifecycle for OverflowTestCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.items = (1..=50)
            .map(|i| format!("条目 {i:02}：用于验证垂直滚动条与 overflow-y 映射").into())
            .collect();
        let (cols, rows) = build_api_table(&[
            ("overflow-x", "auto / hidden / scroll", "水平滚动映射"),
            ("overflow-y", "auto / hidden / scroll", "垂直滚动映射"),
            ("overflow-x-auto", "bool", "等同 overflow-x: auto"),
            ("overflow-y-auto", "bool", "等同 overflow-y: auto"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl OverflowTestCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("overflow_test_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("overflow_test_case.rml.rs").to_string()
    }
}

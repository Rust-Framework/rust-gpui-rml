use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.scroll",
    kind = "case",
    group = "layout",
    order = 85,
)]
#[component]
#[derive(Default)]
pub struct ScrollCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub items: Vec<String>,
}

impl IContribution for ScrollCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.scroll.title")
    }
}

impl ILifecycle for ScrollCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        self.items = (1..=20).map(|i| format!("列表项 {}", i)).collect();

        let (cols, rows) = build_api_table(&[
            (
                "vertical",
                "bool",
                "布尔属性，设置垂直滚动（默认方向）",
            ),
            (
                "horizontal",
                "bool",
                "布尔属性，设置水平滚动",
            ),
            (
                "both",
                "bool",
                "布尔属性，设置双向滚动（垂直 + 水平）",
            ),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ScrollCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("scroll_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("scroll_case.rml.rs").to_string()
    }
}

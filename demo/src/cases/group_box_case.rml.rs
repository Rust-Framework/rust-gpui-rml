use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.group_box",
    kind = "case",
    group = "components",
    order = 67,
)]
#[component]
#[derive(Default)]
pub struct GroupBoxCase {
    pub dynamic_title: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for GroupBoxCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.group_box.title")
    }
}

impl ILifecycle for GroupBoxCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.dynamic_title = "动态标题 1".into();
        let (cols, rows) = build_api_table(&[
            ("title", "string / binding", "分组标题"),
            ("normal / fill / outline", "bool", "3 种样式变体，如 normal=\"\" 或 fill=\"\""),
            ("variant", "string", "样式变体：normal | fill | outline"),
            ("（子节点）", "slot", "分组内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl GroupBoxCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("group_box_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("group_box_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_cycle_title(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.dynamic_title = if self.dynamic_title.contains("1") {
            "动态标题 2".into()
        } else {
            "动态标题 1".into()
        };
    }
}

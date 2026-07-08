use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.collapsible",
    kind = "case",
    group = "components",
    order = 66,
)]
#[component]
#[derive(Default)]
pub struct CollapsibleCase {
    pub is_open: bool,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for CollapsibleCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.collapsible.title")
    }
}

impl ILifecycle for CollapsibleCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.is_open = true;
        let (cols, rows) = build_api_table(&[
            ("open", "bool / 绑定", "展开/折叠状态（默认 false）"),
            ("子节点", "元素", "容器内容（ParentElement）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CollapsibleCase {
    #[computed]
    pub fn state_label(&self) -> &'static str {
        if self.is_open { "已展开" } else { "已折叠" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("collapsible_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("collapsible_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_toggle(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_open = !self.is_open;
    }
}

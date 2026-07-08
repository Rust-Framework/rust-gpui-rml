use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.html",
    kind = "case",
    group = "framework",
    order = 52,
)]
#[component]
#[derive(Default)]
pub struct HtmlCase {
    pub html_content: SharedString,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for HtmlCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.html.title")
    }
}

impl ILifecycle for HtmlCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.html_content = "<p>Hello <strong>RML</strong>!</p>".into();
        let (cols, rows) = build_api_table(&[
            ("html={raw}", "指令", "渲染 HTML 字符串（GPUI 无原生 HTML，降级为 Label 文本）"),
            ("降级行为", "说明", "HTML 标签作为文本字面量显示，不解析"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl HtmlCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("html_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("html_case.rml.rs").to_string()
    }
}

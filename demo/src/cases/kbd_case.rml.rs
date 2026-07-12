use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.kbd",
    kind = "case",
    group = "components",
    order = 60,
)]
#[component]
#[derive(Default)]
pub struct KbdCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for KbdCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.kbd.title")
    }
}

impl ILifecycle for KbdCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("key", "string / binding", "按键组合，如 key=\"cmd-a\" 或 key=\"ctrl-shift-c\""),
            ("outline", "bool", "描边样式（透明背景 + 彩色边框/文字）"),
            ("appearance", "bool", "是否显示默认外观（默认 true，false 时仅显示文本）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl KbdCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("kbd_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("kbd_case.rml.rs").to_string()
    }
}

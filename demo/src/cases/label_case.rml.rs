use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.label",
    kind = "case",
    group = "components",
    order = 23,
)]
#[component]
#[derive(Default)]
pub struct LabelCase {
    pub text: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for LabelCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.label.title")
    }
}

impl ILifecycle for LabelCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.text = "用户名".into();
        let (cols, rows) = build_api_table(&[
            ("label", "string / binding", "标签文本，如 label=\"用户名\""),
            ("（文本子节点）", "slot", "通过子节点设置标签内容"),
            ("text-color / text-size", "string", "文本样式，如 text-color=\"var(--muted)\" text-size=\"0.875rem\""),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl LabelCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("label_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("label_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_cycle_text(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.text = match self.text.as_str() {
            "用户名" => "邮箱地址".into(),
            "邮箱地址" => "密码".into(),
            "密码" => "昵称".into(),
            _ => "用户名".into(),
        };
    }
}

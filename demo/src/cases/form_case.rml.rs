use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.form",
    kind = "case",
    group = "components",
    order = 86,
)]
#[component]
#[derive(Default)]
pub struct FormCase {
    pub form_api_columns: Vec<TableColumn>,
    pub form_api_rows: Vec<TableRow>,
    pub field_api_columns: Vec<TableColumn>,
    pub field_api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for FormCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.form.title")
    }
}

impl ILifecycle for FormCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("horizontal", "bool", "布尔属性，水平布局（标签在左）"),
            ("vertical", "bool", "布尔属性，垂直布局（默认，标签在上）"),
            ("label_width", "string", "标签宽度（像素），如 \"120\" 或 \"120px\"，默认 140"),
            ("label_text_size", "string", "标签文字大小（rems），如 \"0.875\""),
            ("columns", "string", "列数，如 \"2\"，默认 1"),
        ]);
        self.form_api_columns = cols;
        self.form_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("label", "string", "字段标签文本"),
            ("description", "string", "字段描述/帮助文本"),
            ("required", "bool", "布尔属性，标记为必填（显示 *）"),
            ("visible", "bool", "是否可见，默认 true；visible=\"false\" 隐藏"),
            ("label_indent", "bool", "标签缩进，默认 true；label_indent=\"false\" 关闭"),
            ("col_span", "string", "跨列数，如 \"2\"，默认 1"),
            ("col_start", "string", "起始列号"),
            ("col_end", "string", "结束列号"),
        ]);
        self.field_api_columns = cols;
        self.field_api_rows = rows;
    }
}

impl FormCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("form_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("form_case.rml.rs").to_string()
    }
}

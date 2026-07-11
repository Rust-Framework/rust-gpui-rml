use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.textarea",
    kind = "case",
    group = "components",
    order = 36,
)]
#[component]
#[derive(Default)]
pub struct TextareaCase {
    pub bio: String,
    pub remark: String,
    pub single_line: String,
    pub multi_line: String,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TextareaCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.textarea.title")
    }
}

impl ILifecycle for TextareaCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("value", "绑定属性", "双向绑定到 pub String 字段，自动启用 multi_line 模式"),
            ("placeholder", "字符串", "占位文本，传入 InputState builder"),
            ("disabled", "布尔", "禁用状态（disabled 属性）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TextareaCase {
    #[computed]
    fn rml_sample(&self) -> String {
        r#"<textarea value={bio} placeholder="请输入..." />"#
            .to_string()
    }

    #[computed]
    fn rust_sample(&self) -> String {
        r#"#[component]
#[derive(Default)]
pub struct MyView {
    pub bio: String,
}"#
        .to_string()
    }

    #[computed]
    fn char_count(&self) -> usize {
        self.bio.chars().count()
    }

    #[computed]
    fn remark_count(&self) -> usize {
        self.remark.chars().count()
    }
}

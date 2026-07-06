use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

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
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.text = "用户名".into();
        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "标签文本（构造器参数）"),
            ("文本子节点", "字符串", "通过子节点设置标签内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl LabelCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Label label="用户名" />
<Label>用户名</Label>
<Label label={dynamic_title} />"#
            .to_string()
    }
}

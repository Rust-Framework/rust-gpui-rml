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
    pub code_tab: usize,
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
    pub fn rml_sample(&self) -> String {
        r#"<!-- label_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- label 属性：构造器参数 -->
    <Label label="用户名" />

    <!-- 文本子节点：等价于 label 属性 -->
    <Label>用户名</Label>

    <!-- 动态绑定：model 双向绑定 -->
    <input model={text} placeholder="输入标签文本" />
    <Label>{text}</Label>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// label_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct LabelCase {
    pub text: String,
}

impl ILifecycle for LabelCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.text = "用户名".into();
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

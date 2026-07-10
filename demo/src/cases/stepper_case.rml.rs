use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.stepper",
    kind = "case",
    group = "components",
    order = 70,
)]
#[component]
#[derive(Default)]
pub struct StepperCase {
    pub current_step: usize,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for StepperCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.stepper.title")
    }
}

impl ILifecycle for StepperCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.current_step = 0;
        let (cols, rows) = build_api_table(&[
            ("selected-index", "usize / 绑定", "当前选中步骤索引"),
            ("direction", "vertical / horizontal", "布局方向（默认水平）"),
            ("text-center", "布尔标志", "文本居中对齐"),
            ("on-click", "事件", "步骤点击回调（Fn(&usize, ...)）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait）"),
            ("disabled", "bool / 绑定", "禁用"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl StepperCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("stepper_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("stepper_case.rml.rs").to_string()
    }

    // on_click 事件签名为 Fn(&usize, ...)（步骤索引）
    #[command]
    pub fn on_step_click(&mut self, idx: &usize, _cx: &mut Context<Self>) {
        self.current_step = *idx;
    }
}

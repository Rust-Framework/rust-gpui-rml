use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.slot_scope",
    kind = "case",
    group = "framework",
    order = 45,
)]
#[component]
#[derive(Default)]
pub struct SlotScopeCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for SlotScopeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.slot_scope.title")
    }
}

impl ILifecycle for SlotScopeCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("scope={name}", "指令", "插槽作用域变量，如 <template slot=\"x\" scope={panel}>"),
            ("panel.slot_name()", "查询", "返回当前插槽名（如 left / right / bottom）"),
            ("panel.current_size()", "查询", "返回当前尺寸"),
            ("panel.container_size()", "查询", "返回容器总尺寸"),
            ("panel.has_resizable()", "查询", "是否支持拖拽调整大小"),
            ("panel.maximize()", "操作", "最大化此面板"),
            ("panel.restore()", "操作", "还原面板尺寸"),
            ("panel.close()", "操作", "关闭/折叠此面板"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SlotScopeCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("slot_scope_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("slot_scope_case.rml.rs").to_string()
    }
}

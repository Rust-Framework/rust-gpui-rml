use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.radio",
    kind = "case",
    group = "components",
    order = 69,
)]
#[component]
#[derive(Default)]
pub struct RadioCase {
    pub selected_index: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub radio_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for RadioCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.radio.title")
    }
}

impl ILifecycle for RadioCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.selected_index = 0;
        let (cols, group_rows) = build_api_table(&[
            ("selected-index", "number / binding", "当前选中索引（从 0 开始），如 selected-index={idx}"),
            ("horizontal / vertical", "bool", "布局方向，如 horizontal=\"\" 或 vertical=\"\""),
            ("on-click", "event", "选中切换时回调，参数为选中索引"),
            ("disabled", "bool / binding", "禁用整个组"),
            ("（子节点）", "slot", "Radio 元素列表"),
        ]);
        let (_, radio_rows) = build_api_table(&[
            ("label", "string / binding", "Radio 标签文本"),
            ("checked", "bool / binding", "选中状态（由 RadioGroup 管理）"),
            ("disabled", "bool / binding", "禁用"),
            ("tab-index", "number", "Tab 顺序索引"),
            ("tab-stop", "bool", "是否参与 Tab 导航（默认 true）"),
            ("on-click", "event", "点击时回调，参数为切换后的选中状态"),
        ]);
        self.api_columns = cols;
        self.api_rows = group_rows;
        self.radio_rows = radio_rows;
    }
}

impl RadioCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("radio_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("radio_case.rml.rs").to_string()
    }

    // RadioGroup on_click 签名为 Fn(&usize, ...)（选中索引）
    #[command]
    pub fn on_select(&mut self, idx: &usize, _cx: &mut Context<Self>) {
        self.selected_index = *idx;
    }
}

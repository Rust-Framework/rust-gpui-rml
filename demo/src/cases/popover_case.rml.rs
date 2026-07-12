use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.popover",
    kind = "case",
    group = "components",
    order = 62,
)]
#[component]
#[derive(Default)]
pub struct PopoverCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for PopoverCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.popover.title")
    }
}

impl ILifecycle for PopoverCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("anchor", "枚举", "气泡定位锚点：top-left/top-center/top-right/bottom-left/bottom-center/bottom-right/left-center/right-center"),
            ("mouse-button", "枚举", "触发按键：left/right/middle，默认 left"),
            ("appearance", "bool", "是否应用默认样式（bg/border/shadow），默认 true；appearance=false 关闭"),
            ("overlay-closable", "bool", "点击外部是否关闭，默认 true；overlay-closable=false 禁用"),
            ("default-open", "bool", "初始展开状态，默认 false；default-open=true 初始展开"),
            ("slot=trigger", "slot", "触发器元素，如 Button slot=\"trigger\""),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl PopoverCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("popover_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("popover_case.rml.rs").to_string()
    }
}

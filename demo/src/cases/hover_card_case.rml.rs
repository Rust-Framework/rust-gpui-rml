use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.hover_card",
    kind = "case",
    group = "components",
    order = 80,
)]
#[component]
#[derive(Default)]
pub struct HoverCardCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for HoverCardCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.hover_card.title")
    }
}

impl ILifecycle for HoverCardCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("anchor", "枚举", "卡片定位锚点：top-left/top-center/top-right/bottom-left/bottom-center/bottom-right/left-center/right-center"),
            ("appearance", "bool", "是否应用默认样式（bg/border/shadow），默认 true；appearance=false 关闭"),
            ("open-delay", "数值", "鼠标悬浮后显示卡片的延迟（毫秒），默认 500ms"),
            ("close-delay", "数值", "鼠标移开后隐藏卡片的延迟（毫秒），默认 500ms"),
            ("slot=trigger", "slot", "标记 trigger 元素，需实现 Selectable + IntoElement（如 Button）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl HoverCardCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("hover_card_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("hover_card_case.rml.rs").to_string()
    }
}

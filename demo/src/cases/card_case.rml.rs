use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.card",
    kind = "case",
    group = "components",
    order = 30,
)]
#[component]
#[derive(Default)]
pub struct CardCase {
    pub card_title: String,
    pub card_body: String,
    pub hoverable: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for CardCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.card.title")
    }
}

impl ILifecycle for CardCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.card_title = "动态卡片".into();
        self.card_body = "这是通过 value 双向绑定控制的卡片内容。".into();
        self.hoverable = true;
        let (cols, rows) = build_api_table(&[
            ("title", "string / binding", "卡片标题"),
            ("extra", "slot", "标题栏右侧扩展内容"),
            ("cover", "slot", "封面图区域"),
            ("footer", "slot", "底部区域"),
            ("bordered", "bool", "显示边框"),
            ("borderless", "bool", "无边框样式"),
            ("hoverable", "bool", "悬浮高亮效果"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CardCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("card_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("card_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_toggle_hoverable(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.hoverable = !self.hoverable;
    }
}

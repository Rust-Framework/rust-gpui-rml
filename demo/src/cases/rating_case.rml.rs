use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.rating",
    kind = "case",
    group = "components",
    order = 71,
)]
#[component]
#[derive(Default)]
pub struct RatingCase {
    pub rating_value: usize,
    pub max_stars: usize,
    pub is_readonly: bool,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for RatingCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.rating.title")
    }
}

impl ILifecycle for RatingCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.rating_value = 3;
        self.max_stars = 5;
        self.is_readonly = false;
        let (cols, rows) = build_api_table(&[
            ("value", "usize / 绑定", "当前评分值（0..=max）"),
            ("max", "usize", "最大星数（默认 5）"),
            ("color", "主题色名", "星标激活色（如 red/yellow/green）"),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("disabled", "bool / binding", "禁用交互"),
            ("on-click", "event", "点击星标时回调，参数为评分值"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl RatingCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("rating_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("rating_case.rml.rs").to_string()
    }

    // on_click 事件签名为 Fn(&usize, ...)（评分值）
    #[command]
    pub fn on_rating_change(&mut self, value: &usize, _cx: &mut Context<Self>) {
        self.rating_value = *value;
    }
}

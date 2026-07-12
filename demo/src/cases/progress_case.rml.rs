use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.progress",
    kind = "case",
    group = "components",
    order = 26,
)]
#[component]
#[derive(Default)]
pub struct ProgressCase {
    pub current: f32,
    pub loading: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ProgressCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.progress.title")
    }
}

impl ILifecycle for ProgressCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.current = 60.0;
        let (cols, rows) = build_api_table(&[
            ("value", "number / binding", "进度值 0–100（自动 clamp）"),
            ("loading", "bool / binding", "加载中状态（显示无限滑动动画，忽略 value）"),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ProgressCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.loading {
            "加载中... (loading=true)".to_string()
        } else {
            format!("当前进度：{:.0}%", self.current)
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("progress_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("progress_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_increase(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current = (self.current + 10.0).min(100.0);
    }

    #[command]
    pub fn on_decrease(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current = (self.current - 10.0).max(0.0);
    }

    #[command]
    pub fn on_toggle_loading(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.loading = !self.loading;
    }
}

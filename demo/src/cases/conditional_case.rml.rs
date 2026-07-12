use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.conditional",
    kind = "case",
    group = "framework",
    order = 42,
)]
#[component]
#[derive(Default)]
pub struct ConditionalCase {
    pub tab_index: u8,
    pub show_detail: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ConditionalCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.conditional.title")
    }
}

impl ILifecycle for ConditionalCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.tab_index = 0;
        self.show_detail = true;
        let (cols, rows) = build_api_table(&[
            ("if={expr}", "指令", "条件为真时渲染元素；支持取反如 if={!flag}"),
            ("else-if={expr}", "指令", "多分支链，跟在 if / else-if 后"),
            ("else", "指令", "默认分支，跟在 if / else-if 后"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ConditionalCase {
    #[computed]
    pub fn tab_label(&self) -> &'static str {
        match self.tab_index {
            0 => "概览",
            1 => "详情",
            _ => "设置",
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("conditional_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("conditional_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_tab_overview(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.tab_index = 0;
    }

    #[command]
    pub fn on_tab_detail(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.tab_index = 1;
    }

    #[command]
    pub fn on_tab_settings(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.tab_index = 2;
    }

    #[command]
    pub fn on_toggle_detail(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_detail = !self.show_detail;
    }
}

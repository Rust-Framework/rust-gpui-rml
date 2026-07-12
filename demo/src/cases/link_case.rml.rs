use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.link",
    kind = "case",
    group = "components",
    order = 65,
)]
#[component]
#[derive(Default)]
pub struct LinkCase {
    pub dynamic_url: String,
    pub click_count: u32,
    pub is_disabled: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for LinkCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.link.title")
    }
}

impl ILifecycle for LinkCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.dynamic_url = "https://github.com".into();
        let (cols, rows) = build_api_table(&[
            ("href", "string / binding", "目标 URL，如 href=\"https://example.com\""),
            ("on-click", "event", "点击时回调"),
            ("disabled", "bool / binding", "禁用链接交互"),
            ("（子节点）", "slot", "链接文本或内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl LinkCase {
    #[computed]
    pub fn dynamic_label(&self) -> &'static str {
        if self.dynamic_url.contains("github") { "GitHub 主页" } else { "crates.io 主页" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("link_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("link_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_cycle_url(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.dynamic_url = if self.dynamic_url.contains("github") {
            "https://crates.io".into()
        } else {
            "https://github.com".into()
        };
    }

    #[command]
    pub fn on_link_click(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.click_count += 1;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }
}

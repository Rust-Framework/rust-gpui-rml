use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.show",
    kind = "case",
    group = "framework",
    order = 54,
)]
#[component]
#[derive(Default)]
pub struct ShowCase {
    pub show_card: bool,
    pub if_card: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ShowCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.show.title")
    }
}

impl ILifecycle for ShowCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.show_card = true;
        self.if_card = true;
        let (cols, rows) = build_api_table(&[
            ("show={cond}", "指令", "cond 为 false 时元素不可见但保留布局空间（Visibility::Hidden）"),
            ("if={cond}", "对比", "cond 为 false 时元素不渲染，不占布局空间（Display::None）"),
            ("优先级", "说明", "if 优先于 show（同时存在时 show 被忽略）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ShowCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("show_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("show_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_toggle_show(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_card = !self.show_card;
    }

    #[command]
    pub fn on_toggle_if(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.if_card = !self.if_card;
    }
}

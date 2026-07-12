use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.spinner",
    kind = "case",
    group = "components",
    order = 64,
)]
#[component]
#[derive(Default)]
pub struct SpinnerCase {
    pub is_loading: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub skeleton_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for SpinnerCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.spinner.title")
    }
}

impl ILifecycle for SpinnerCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.is_loading = true;
        let (cols, rows) = build_api_table(&[
            ("icon", "string", "自定义图标，如 icon=\"Bell\"（默认 Loader）"),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;

        let (_, skel_rows) = build_api_table(&[
            ("secondary", "bool", "切换为次级颜色（次要占位）"),
        ]);
        self.skeleton_rows = skel_rows;
    }
}

impl SpinnerCase {
    #[computed]
    pub fn loading_label(&self) -> &'static str {
        if self.is_loading { "加载中" } else { "已完成" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("spinner_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("spinner_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_toggle_loading(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_loading = !self.is_loading;
    }
}

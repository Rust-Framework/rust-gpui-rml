use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.skeleton",
    kind = "case",
    group = "components",
    order = 91,
)]
#[component]
#[derive(Default)]
pub struct SkeletonCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for SkeletonCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.skeleton.title")
    }
}

impl ILifecycle for SkeletonCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("secondary", "布尔标志", "切换为次级颜色（更浅的灰色），适用于多层骨架屏视觉层次区分"),
            ("style", "CSS 字符串", "内联样式，控制宽高/圆角等（如 style=\"width: 200px; height: 20px;\"）"),
            ("class", "字符串", "CSS class 名称，用于样式定制"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SkeletonCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("skeleton_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("skeleton_case.rml.rs").to_string()
    }
}

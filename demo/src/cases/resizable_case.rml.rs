use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.resizable",
    kind = "case",
    group = "components",
    order = 74,
)]
#[component]
#[derive(Default)]
pub struct ResizableCase {
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ResizableCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.resizable.title")
    }
}

impl ILifecycle for ResizableCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("direction", "horizontal/vertical", "方向（默认 horizontal，选择 h/v_resizable 构造器）"),
            ("size", "px/绑定", "组的交叉轴尺寸（horizontal 为高度，vertical 为宽度）"),
            ("on-resize", "event", "面板尺寸调整时回调"),
            ("resizable-panel", "slot", "面板子节点"),
            ("size", "string / binding", "面板初始尺寸，如 size=\"400px\""),
            ("size-range", "string", "面板尺寸范围，如 size-range=\"200px..800px\""),
            ("visible", "bool", "面板可见性（默认 true）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ResizableCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("resizable_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("resizable_case.rml.rs").to_string()
    }
}

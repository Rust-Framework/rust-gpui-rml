use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.sheet",
    kind = "case",
    group = "components",
    order = 81,
)]
#[component]
#[derive(Default)]
pub struct SheetCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for SheetCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.sheet.title")
    }
}

impl ILifecycle for SheetCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("title", "string", "抽屉标题文本，渲染在标题栏左侧"),
            ("footer", "string", "抽屉页脚文本，渲染在底部区域"),
            ("size", "长度", "抽屉面板尺寸，支持 px/百分比/裸数字，如 350px / 50% / 400"),
            ("resizable", "bool", "是否可拖拽调整大小，默认 true；resizable=false 禁用"),
            ("overlay", "bool", "是否显示背景遮罩，默认 true；overlay=false 关闭"),
            ("overlay-closable", "bool", "点击遮罩是否关闭抽屉，默认 true；overlay-closable=false 禁用"),
            ("on-close", "event", "抽屉关闭时回调"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SheetCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("sheet_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("sheet_case.rml.rs").to_string()
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};
use rml_ui_term::TerminalView;

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.terminal",
    kind = "case",
    group = "components",
    order = 39,
)]
#[component]
#[derive(Default)]
pub struct TerminalCase {
    /// EntityRef 组件字段：Option<Entity<TerminalView>>。
    /// 在 on_loaded 中通过 cx.new + spawn_default 创建。
    /// codegen 生成 self.term.as_ref().expect("init term in on_loaded").clone()
    pub term: Option<gpui::Entity<TerminalView>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TerminalCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.terminal.title")
    }
}

impl ILifecycle for TerminalCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let term = cx.new(|cx| TerminalView::spawn_default(cx));
        self.term = Some(term);

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段（Option<Entity<TerminalView>>），如 ref=\"term\""),
            ("on_loaded 创建", "命令式 API", "在 on_loaded 中通过 cx.new(|cx| TerminalView::spawn_default(cx)) 创建 Entity，赋值到同名字段"),
            ("style / class", "string", "CSS 样式属性，如 style=\"height: 400px\" 确保终端有足够渲染空间"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TerminalCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("terminal_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("terminal_case.rml.rs").to_string()
    }
}

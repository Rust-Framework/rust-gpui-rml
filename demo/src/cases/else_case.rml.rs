use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "framework.else",
    kind = "case",
    group = "framework",
    order = 50,
)]
#[component]
#[derive(Default)]
pub struct ElseCase {
    pub show_a: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ElseCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.else.title")
    }
}

impl ILifecycle for ElseCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.show_a = true;
        let (cols, rows) = build_api_table(&[
            ("if={cond}", "指令", "条件为真时渲染此分支"),
            ("else", "指令", "与同父的 if 配对，条件为假时渲染此分支"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ElseCase {
    #[command]
    pub fn on_toggle(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_a = !self.show_a;
    }
}

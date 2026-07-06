use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.kbd",
    kind = "case",
    group = "components",
    order = 60,
)]
#[component]
#[derive(Default)]
pub struct KbdCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for KbdCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.kbd.title")
    }
}

impl ILifecycle for KbdCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let _ = (_window, _cx);
        let (cols, rows) = build_api_table(&[
            ("key", "字符串", "按键组合（如 cmd-a / ctrl-shift-c），由 Keystroke::parse 解析"),
            ("outline", "布尔", "使用 outline 样式（默认 false）"),
            ("appearance", "布尔", "是否显示外观（默认 true，false 时仅显示文本）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

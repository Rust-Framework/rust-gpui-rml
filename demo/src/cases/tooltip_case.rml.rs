use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.tooltip",
    kind = "case",
    group = "components",
    order = 61,
)]
#[component]
#[derive(Default)]
pub struct TooltipCase {
    pub tooltip_text: SharedString,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl TooltipCase {
    #[computed]
    pub fn dynamic_tooltip(&self) -> SharedString {
        self.tooltip_text.clone()
    }
}

impl IContribution for TooltipCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tooltip.title")
    }
}

impl ILifecycle for TooltipCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let _ = (_window, _cx);
        self.tooltip_text = "动态 Tooltip 内容".into();
        let (cols, rows) = build_api_table(&[
            ("tooltip", "字符串", "悬浮提示文本，生成 .tooltip(\"text\")，仅支持特定组件"),
            ("支持组件", "枚举", "Button / IconButton / DropdownButton / Toggle / Checkbox / Clipboard / Radio / Switch"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

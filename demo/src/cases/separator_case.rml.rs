use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.separator",
    kind = "case",
    group = "components",
    order = 24,
)]
#[component]
#[derive(Default)]
pub struct SeparatorCase {
    pub is_vertical: bool,
    pub is_dashed: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for SeparatorCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.separator.title")
    }
}

impl ILifecycle for SeparatorCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("vertical", "布尔标志", "垂直方向"),
            ("dashed", "布尔标志", "虚线样式"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SeparatorCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Separator />
<Separator vertical="" />
<Separator dashed="" />"#
            .to_string()
    }

    #[command]
    pub fn on_toggle_orientation(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.is_vertical = !self.is_vertical;
    }

    #[command]
    pub fn on_toggle_dashed(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.is_dashed = !self.is_dashed;
    }
}

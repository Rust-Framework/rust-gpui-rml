use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.switch",
    kind = "case",
    group = "components",
    order = 34,
)]
#[component]
#[derive(Default)]
pub struct SwitchCase {
    pub is_on: bool,
    pub is_disabled: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for SwitchCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.switch.title")
    }
}

impl ILifecycle for SwitchCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "标签文本"),
            ("checked", "布尔", "开关状态"),
            ("disabled", "布尔", "禁用"),
            ("size", "small/medium/large", "尺寸"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SwitchCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.is_on {
            "当前：开启".to_string()
        } else {
            "当前：关闭".to_string()
        }
    }

    #[command]
    pub fn on_toggle(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_on = !self.is_on;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
        cx.notify();
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.checkbox",
    kind = "case",
    group = "components",
    order = 33,
)]
#[component]
#[derive(Default)]
pub struct CheckboxCase {
    pub is_checked: bool,
    pub is_disabled: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for CheckboxCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.checkbox.title")
    }
}

impl ILifecycle for CheckboxCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "标签文本"),
            ("checked", "布尔", "勾选状态"),
            ("disabled", "布尔", "禁用"),
            ("size", "small/medium/large", "尺寸"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CheckboxCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.is_checked {
            "当前：已勾选".to_string()
        } else {
            "当前：未勾选".to_string()
        }
    }

    #[command]
    pub fn on_toggle_checked(&mut self, checked: &bool, cx: &mut Context<Self>) {
        self.is_checked = *checked;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_checked_button(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_checked = !self.is_checked;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
        cx.notify();
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

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
    pub wifi_on: bool,
    pub bluetooth_on: bool,
    pub dark_mode: bool,
    pub auto_sync: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.wifi_on = true;
        self.dark_mode = true;
        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "标签文本"),
            ("checked", "布尔/绑定", "开关状态"),
            ("disabled", "布尔/绑定", "禁用"),
            ("tooltip", "字符串", "悬浮提示"),
            ("on-click", "事件", "点击回调（Fn(&bool, ...)）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SwitchCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.is_on {
            "开启".to_string()
        } else {
            "关闭".to_string()
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("switch_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("switch_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_toggle(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.is_on = *checked;
    }

    #[command]
    pub fn on_toggle_button(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_on = !self.is_on;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_toggle_wifi(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.wifi_on = *checked;
    }

    #[command]
    pub fn on_toggle_bluetooth(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.bluetooth_on = *checked;
    }

    #[command]
    pub fn on_toggle_dark(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.dark_mode = *checked;
    }

    #[command]
    pub fn on_toggle_sync(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.auto_sync = *checked;
    }
}

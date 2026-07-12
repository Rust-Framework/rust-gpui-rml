use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

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
    pub email_notify: bool,
    pub sms_notify: bool,
    pub push_notify: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.email_notify = true;
        let (cols, rows) = build_api_table(&[
            ("label", "string / binding", "标签文本"),
            ("checked", "bool / binding", "勾选状态"),
            ("disabled", "bool / binding", "禁用"),
            ("tooltip", "string", "悬浮提示"),
            ("on-click", "event", "点击时回调，参数为切换后的勾选状态"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CheckboxCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.is_checked {
            "已勾选".to_string()
        } else {
            "未勾选".to_string()
        }
    }

    #[computed]
    pub fn notify_summary(&self) -> String {
        let mut items: Vec<&str> = Vec::new();
        if self.email_notify {
            items.push("邮件");
        }
        if self.sms_notify {
            items.push("短信");
        }
        if self.push_notify {
            items.push("推送");
        }
        if items.is_empty() {
            "无".to_string()
        } else {
            items.join("、")
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("checkbox_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("checkbox_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_toggle_checked(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.is_checked = *checked;
    }

    #[command]
    pub fn on_toggle_checked_button(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_checked = !self.is_checked;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_toggle_email(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.email_notify = *checked;
    }

    #[command]
    pub fn on_toggle_sms(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.sms_notify = *checked;
    }

    #[command]
    pub fn on_toggle_push(&mut self, checked: &bool, _cx: &mut Context<Self>) {
        self.push_notify = *checked;
    }
}

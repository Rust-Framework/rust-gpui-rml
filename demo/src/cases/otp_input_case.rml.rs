use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{OtpState, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.otp_input",
    kind = "case",
    group = "components",
    order = 72,
)]
#[component]
#[derive(Default)]
pub struct OtpInputCase {
    pub otp_value: String,
    pub otp_input: ElementRef<OtpState>,
    /// 无 ref 的 OtpInput 回退字段，on_loaded 中初始化
    pub otp_state: Option<gpui::Entity<OtpState>>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for OtpInputCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.otp_input.title")
    }
}

impl ILifecycle for OtpInputCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.otp_state = Some(cx.new(|cx| OtpState::new(6, _window, cx)));
        let (cols, rows) = build_api_table(&[
            ("length", "usize", "OTP 位数（默认 6，注入 state_ctor）"),
            ("groups", "usize", "分组数（默认 2，组间间隔更大）"),
            ("masked", "bool", "掩码显示（注入 state_ctor）"),
            ("default-value", "字符串", "默认值（注入 state_ctor）"),
            ("disabled", "bool / 绑定", "禁用（Disableable trait）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait）"),
            ("ref", "字符串（指令）", "元素引用名，绑定到 ElementRef<OtpState>"),
            ("on-change", "事件", "内容变化回调（参数：&Entity<OtpState>）"),
            ("on-focus", "事件", "获得焦点回调（InputEvent::Focus）"),
            ("on-blur", "事件", "失去焦点回调（InputEvent::Blur）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl OtpInputCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("otp_input_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("otp_input_case.rml.rs").to_string()
    }

    /// on_change 事件参数为 &Entity<OtpState>（通过 cx.subscribe 订阅 InputEvent::Change；
    /// 传 Entity 句柄而非 &OtpState，避免 entity.read(cx) 与后续 cx 可变借用冲突）
    #[command]
    pub fn on_otp_change(&mut self, entity: &gpui::Entity<OtpState>, _cx: &mut Context<Self>) {
        self.otp_value = entity.read(_cx).value().to_string();
    }
}

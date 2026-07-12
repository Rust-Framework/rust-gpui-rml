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
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段，如 ref=\"otp_input\""),
            ("length", "number", "OTP 位数（默认 6），如 length=\"6\""),
            ("groups", "number", "分组数（默认 2），组间间隔更大"),
            ("masked", "bool", "掩码显示（默认 false）"),
            ("default-value", "string", "初始值，如 default-value=\"123456\""),
            ("disabled", "bool / binding", "禁用状态"),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("on-change", "event", "验证码变化时回调"),
            ("on-focus", "event", "获得焦点时回调"),
            ("on-blur", "event", "失去焦点时回调"),
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

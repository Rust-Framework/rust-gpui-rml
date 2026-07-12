use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.alert",
    kind = "case",
    group = "components",
    order = 40,
)]
#[component]
#[derive(Default)]
pub struct AlertCase {
    /// 控制 on-close + if 条件渲染演示中两个 Alert 的可见性
    pub is_visible: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for AlertCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.alert.title")
    }
}

impl ILifecycle for AlertCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.is_visible = true;

        let (cols, rows) = build_api_table(&[
            ("info / success / warning / error", "bool", "4 种样式变体，如 info=\"\" 或 success=\"\""),
            ("variant", "string", "样式变体：default | info | success | warning | error"),
            ("message", "string / binding", "提示内容，如 message=\"操作成功\" 或 message={msg}"),
            ("title", "string / binding", "提示标题"),
            ("banner", "bool", "顶部横幅模式（无边框，撑满宽度）"),
            ("visible", "bool / binding", "是否可见"),
            ("icon", "string", "自定义图标，如 icon=\"Bell\""),
            ("on-close", "event", "点击关闭按钮时回调"),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("if", "指令", "条件渲染，如 if={is_visible}"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl AlertCase {
    /// message 绑定演示：message={message_from_field} 走绑定路径
    #[computed]
    pub fn message_from_field(&self) -> &'static str {
        "绑定字段作为 message（第二优先级）"
    }

    /// 当前可见状态的可读文本，配合 if={is_visible} 演示条件渲染
    #[computed]
    pub fn visibility_label(&self) -> &'static str {
        if self.is_visible { "可见" } else { "已关闭" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("alert_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("alert_case.rml.rs").to_string()
    }

    /// on-close 回调签名：(&ClickEvent, &mut Context<Self>)
    /// 由 alert.rs 中的 gen_on_close_setter 生成 .on_close(cx.listener(...))
    #[command]
    pub fn on_close(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_visible = false;
    }

    /// 重置 is_visible 为 true，让被关闭的 Alert 重新显示
    #[command]
    pub fn on_reset(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_visible = true;
    }
}

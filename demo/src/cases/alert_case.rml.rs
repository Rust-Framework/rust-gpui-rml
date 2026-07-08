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
            ("info / success / warning / error", "布尔标志", "4 种 variant 关联函数（构造器选择 Alert::info(id, msg) 等）"),
            ("variant", "default/info/success/warning/error", "variant 属性（builder 方法 .with_variant(AlertVariant::Info)）"),
            ("message", "String / 绑定", "提示内容（构造器参数，优先级：静态属性 > 绑定 > 文本子节点）"),
            ("title", "String", "提示标题"),
            ("banner", "布尔标志", "切换为顶部横幅模式（无边框，撑满宽度）"),
            ("visible", "bool / 绑定", "是否可见"),
            ("icon", "IconName 枚举变体名", "自定义图标（如 icon=\"Bell\"）"),
            ("on-close", "事件", "关闭按钮点击回调（ClickEvent）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait）"),
            ("if", "指令", "条件渲染指令（if={expr}，false 时元素不渲染）"),
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

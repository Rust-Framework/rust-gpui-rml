use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.notification",
    kind = "case",
    group = "components",
    order = 84,
)]
#[component]
#[derive(Default)]
pub struct NotificationCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for NotificationCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.notification.title")
    }
}

impl ILifecycle for NotificationCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("title", "string", "通知标题文本，可选（省略时仅显示消息）"),
            ("message", "string", "通知消息内容"),
            (
                "success",
                "bool",
                "布尔属性，设置通知类型为 Success（绿色图标）",
            ),
            (
                "info",
                "bool",
                "布尔属性，设置通知类型为 Info（蓝色图标，默认）",
            ),
            (
                "warning",
                "bool",
                "布尔属性，设置通知类型为 Warning（黄色图标）",
            ),
            (
                "error",
                "bool",
                "布尔属性，设置通知类型为 Error（红色图标）",
            ),
            (
                "autohide",
                "bool",
                "是否自动隐藏，默认 true；autohide=false 持续显示直到手动关闭",
            ),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl NotificationCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("notification_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("notification_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_info(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.__rml_notify_info("这是一条信息通知（来自命令回调）");
    }

    #[command]
    pub fn on_success(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.__rml_notify_success("操作成功完成（来自命令回调）");
    }

    #[command]
    pub fn on_warning(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.__rml_notify_warning("请注意潜在风险（来自命令回调）");
    }

    #[command]
    pub fn on_error(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.__rml_notify_error("操作执行失败（来自命令回调）");
    }
}

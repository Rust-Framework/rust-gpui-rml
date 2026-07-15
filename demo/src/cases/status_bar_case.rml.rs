use std::sync::Once;

use gpui::SharedString;
use rml::prelude::*;
use rml_core::contribution::register_visual_ability;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.status_bar",
    kind = "case",
    group = "components",
    order = 14,
)]
#[component]
#[derive(Default)]
pub struct StatusBarCase {
    pub last_action: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for StatusBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.status_bar.title")
    }
}

impl ILifecycle for StatusBarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("kind = \"status\"", "贡献类型", "注册到状态栏插槽"),
            ("host-id", "string", "宿主标识"),
            ("order", "number", "状态栏排序"),
            ("name", "string", "状态栏显示文案"),
            ("render", "方法", "自定义状态栏渲染（code-behind）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl StatusBarCase {
    #[computed]
    pub fn action_status(&self) -> String {
        if self.last_action.is_empty() {
            "尚未触发任何操作".to_string()
        } else {
            format!("上次操作：{}", self.last_action)
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("status_bar_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("status_bar_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_show_ready(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "查看就绪状态".to_string();
    }

    #[command]
    pub fn on_show_case(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "查看案例状态".to_string();
    }
}

/// 状态栏贡献：演示 status slot（从 shell_meta.rs 迁入）
#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[component]
#[derive(Default)]
pub struct StatusReady {}

impl IContribution for StatusReady {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.status_ready")
    }
}

static STATUS_READY_REGISTERED: Once = Once::new();

/// 注册 `StatusReady` 的 `IVisual` 能力 cast。
pub fn ensure_status_ready_registered() {
    STATUS_READY_REGISTERED.call_once(|| {
        register_visual_ability::<StatusReady>();
    });
}

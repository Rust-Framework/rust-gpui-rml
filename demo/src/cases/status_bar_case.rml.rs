use std::sync::Once;

use gpui::{AnyElement, ParentElement, SharedString, Styled};
use rml::prelude::*;
use rml_core::contribution::{register_visual_ability, IVisual};
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

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
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("kind = \"status\"", "贡献类型", "注册到状态栏插槽"),
            ("host_id", "字符串", "宿主标识"),
            ("order", "数字", "状态栏排序"),
            ("IContribution::name", "方法", "状态栏显示文案"),
            ("IVisual::render", "方法", "自定义状态栏渲染"),
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
        r#"<!-- status_bar_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- StatusBar 案例演示贡献点机制（kind = "status"） -->
    <p>请查看窗口底部状态栏，左侧 "就绪" 由 StatusReady 贡献点渲染。</p>
    <p>{action_status}</p>
    <Button label="查看就绪状态" on-click={on_show_ready} />
    <Button label="查看案例状态" on-click={on_show_case} />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// status_bar_case.rml.rs：状态栏贡献点（kind = "status"）
use gpui::{AnyElement, ParentElement, SharedString, Styled};
use rml::prelude::*;
use rml_core::contribution::IVisual;

// 通过 #[contribute(kind = "status")] 注册到宿主 shell 的状态栏插槽
#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IContribution for StatusReady {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("shell.status_ready") }
}

// IVisual::render 自定义状态栏渲染内容
impl IVisual for StatusReady {
    fn render(&self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> AnyElement {
        gpui::div()
            .text_xs()
            .child(t_static("shell.status_ready"))
            .into_any_element()
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_show_ready(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "查看就绪状态".to_string();
    }

    #[command]
    pub fn on_show_case(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "查看案例状态".to_string();
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

/// 状态栏贡献：演示 status slot（从 shell_meta.rs 迁入）
#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IContribution for StatusReady {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.status_ready")
    }
}

impl IVisual for StatusReady {
    fn render(&self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> AnyElement {
        gpui::div()
            .text_xs()
            .child(t_static("shell.status_ready"))
            .into_any_element()
    }
}

static STATUS_READY_REGISTERED: Once = Once::new();

/// 注册 `StatusReady` 的 `IVisual` 能力 cast。
///
/// `StatusReady` 有 `#[contribute]` 无 `#[component]`，视觉能力不自动注册。
/// 需在 `MainWindow::on_loaded` 的 `project_entries()` 前调用，使 `as_visual()` 查询生效。
pub fn ensure_status_ready_registered() {
    STATUS_READY_REGISTERED.call_once(|| {
        register_visual_ability::<StatusReady>();
    });
}

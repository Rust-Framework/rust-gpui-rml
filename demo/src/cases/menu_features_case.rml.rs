use std::sync::Arc;

use gpui::SharedString;
use rml::prelude::*;
use rml_core::command::RelayCommand;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.menu.features",
    kind = "case",
    group = "menu",
    order = 19,
)]
#[component]
#[derive(Default)]
pub struct MenuFeaturesCase {
    pub is_checked: bool,
    pub last_action: String,
    /// B-1 demo：声明式命令绑定字段。
    /// RML `<menu-item command={save_command} />` 据此生成 clone-Arc-out 闭包，
    /// 点击时经 `ICommand::execute` 调度（区别于 `on-click={method}` 的强类型直接调用）。
    /// 类型为 `Arc<RelayCommand>`（具体类型）而非 `Arc<dyn ICommand>`，以便
    /// `#[derive(Default)]` 生效——框架已为 `RelayCommand` 实现 `Default`（no-op 空对象）。
    pub save_command: Arc<RelayCommand>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for MenuFeaturesCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.features.title")
    }
}

impl ILifecycle for MenuFeaturesCase {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        // 初始化 save_command：点击时设置 last_action（复用现有 status 展示链路）
        self.save_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.last_action = "Save command executed".to_string();
            cx.notify();
        }));
        let (cols, rows) = build_api_table(&[
            ("scrollable", "布尔标志", "启用滚动"),
            ("max-h", "数字", "最大高度（像素）"),
            ("menu-item disabled", "布尔标志", "禁用项"),
            ("menu-item checked", "布尔", "勾选状态"),
            ("menu-item href", "URL", "外链跳转"),
            ("menu-item icon", "图标名", "菜单项图标"),
            ("menu-item header", "布尔标志", "分组标题"),
            ("menu-item 子节点", "menu-item", "子菜单"),
            ("menu-item command", "Arc<RelayCommand>", "声明式命令绑定"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl MenuFeaturesCase {
    #[computed]
    pub fn features_status(&self) -> String {
        self.last_action.clone()
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("menu_features_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("menu_features_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_available(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Available".to_string();
    }

    #[command]
    pub fn on_disabled(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Disabled (should not fire)".to_string();
    }

    #[command]
    pub fn on_toggle_check(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.is_checked = !self.is_checked;
        self.last_action = format!("Checked: {}", self.is_checked);
    }

    #[command]
    pub fn on_nested_a(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Nested A".to_string();
    }

    #[command]
    pub fn on_nested_b(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Nested B".to_string();
    }
}

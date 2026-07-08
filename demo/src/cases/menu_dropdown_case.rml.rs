use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.menu.dropdown",
    kind = "case",
    group = "menu",
    order = 17,
)]
#[component]
#[derive(Default)]
pub struct MenuDropdownCase {
    pub last_action: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for MenuDropdownCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.dropdown.title")
    }
}

impl ILifecycle for MenuDropdownCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("anchor", "枚举", "弹出锚点位置"),
            ("第一个子节点", "组件", "触发器（通常 Button）"),
            ("menu-item label", "字符串", "菜单项文案"),
            ("menu-item icon", "图标名", "菜单项图标"),
            ("menu-item on-click", "事件", "点击回调"),
            ("menu-separator", "标签", "分组分隔线"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl MenuDropdownCase {
    #[computed]
    pub fn dropdown_status(&self) -> String {
        if self.last_action.is_empty() {
            rml_core::i18n::t_static("case.menu.dropdown.idle").to_string()
        } else {
            self.last_action.clone()
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("menu_dropdown_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("menu_dropdown_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_custom(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Custom Action".to_string();
    }

    #[command]
    pub fn on_standard(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Standard Action".to_string();
    }

    #[command]
    pub fn on_exit(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Exit".to_string();
    }
}

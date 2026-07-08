use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.menu.context",
    kind = "case",
    group = "menu",
    order = 16,
)]
#[component]
#[derive(Default)]
pub struct MenuContextCase {
    pub last_action: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for MenuContextCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.context.title")
    }
}

impl ILifecycle for MenuContextCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("menu-item label", "字符串", "菜单项文案"),
            ("menu-item icon", "图标名", "菜单项图标"),
            ("menu-item on-click", "事件", "点击回调"),
            ("menu-separator", "标签", "分组分隔线"),
            ("menu-item 子节点", "menu-item", "子菜单"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl MenuContextCase {
    #[computed]
    pub fn context_status(&self) -> String {
        if self.last_action.is_empty() {
            rml_core::i18n::t_static("case.menu.context.idle").to_string()
        } else {
            format!(
                "{}: {}",
                rml_core::i18n::t_static("case.menu.last_action"),
                self.last_action
            )
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("menu_context_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("menu_context_case.rml.rs").to_string()
    }

    fn set_action(&mut self, name: &str) {
        self.last_action = name.to_string();
    }

    #[command]
    pub fn on_open(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Open");
    }

    #[command]
    pub fn on_copy(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Copy");
    }

    #[command]
    pub fn on_cut(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Cut");
    }

    #[command]
    pub fn on_paste(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Paste");
    }

    #[command]
    pub fn on_new_file(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("New File");
    }

    #[command]
    pub fn on_new_folder(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("New Folder");
    }

    #[command]
    pub fn on_delete(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.set_action("Delete");
    }
}

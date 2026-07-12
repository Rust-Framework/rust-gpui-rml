use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.list",
    kind = "case",
    group = "framework",
    order = 43,
)]
#[component]
#[derive(Default)]
pub struct ListCase {
    pub items: Vec<SharedString>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ListCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.list.title")
    }
}

impl ILifecycle for ListCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.items = vec![
            "Rust".into(),
            "GPUI".into(),
            "RML".into(),
            "Component".into(),
            "Binding".into(),
        ];
        let (cols, rows) = build_api_table(&[
            ("each={x in items}", "指令", "遍历列表渲染每个元素，如 each={item in items}"),
            ("items 字段", "binding", "ViewModel 中的列表数据源"),
            ("修改后重渲", "说明", "修改列表字段后框架自动触发重渲"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ListCase {
    #[computed]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("list_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("list_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_add_item(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let idx = self.items.len() + 1;
        self.items.push(format!("Item {}", idx).into());
        self.__rml_bump_version("items");
    }

    #[command]
    pub fn on_remove_item(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.items.pop();
        self.__rml_bump_version("items");
    }
}

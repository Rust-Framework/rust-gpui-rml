use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[derive(Clone, Default)]
pub struct KeyItem {
    pub id: SharedString,
    pub label: SharedString,
}

#[contribute(
    host_id = "demo.shell",
    id = "framework.key",
    kind = "case",
    group = "framework",
    order = 53,
)]
#[component]
#[derive(Default)]
pub struct KeyCase {
    pub items: Vec<KeyItem>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for KeyCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.key.title")
    }
}

impl ILifecycle for KeyCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.items = vec![
            KeyItem {
                id: "i1".into(),
                label: "第一项".into(),
            },
            KeyItem {
                id: "i2".into(),
                label: "第二项".into(),
            },
            KeyItem {
                id: "i3".into(),
                label: "第三项".into(),
            },
        ];
        let (cols, rows) = build_api_table(&[
            ("key={expr}", "指令", "为 each 项提供稳定标识，如 key={item.id}"),
            ("key 优先级", "说明", "ref > key > 事件处理器"),
            ("key 表达式作用域", "说明", "each 作用域内引用循环变量，如 key={item.id}"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl KeyCase {
    #[computed]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("key_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("key_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_prepend(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let idx = self.items.len() + 1;
        self.items.insert(
            0,
            KeyItem {
                id: format!("i{}", idx).into(),
                label: format!("插入项 {}", idx).into(),
            },
        );
        self.__rml_bump_version("items");
    }

    #[command]
    pub fn on_clear(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.items.clear();
        self.__rml_bump_version("items");
    }
}

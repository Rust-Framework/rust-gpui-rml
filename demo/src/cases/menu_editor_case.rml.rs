use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.menu.editor",
    kind = "case",
    group = "menu",
    order = 18,
)]
#[component]
#[derive(Default)]
pub struct MenuEditorCase {
    pub word_wrap: bool,
    pub last_action: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for MenuEditorCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.editor.title")
    }
}

impl ILifecycle for MenuEditorCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("check-side", "枚举", "勾选标记位置（Right/Left）"),
            ("menu-item checked", "bool", "勾选状态绑定"),
            ("menu-item label", "string", "菜单项文案"),
            ("menu-item on-click", "event", "点击回调"),
            ("menu-separator", "标签", "分组分隔线"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl MenuEditorCase {
    #[computed]
    pub fn editor_status(&self) -> String {
        if self.last_action.is_empty() {
            format!(
                "{}: {}",
                rml_core::i18n::t_static("case.menu.word_wrap"),
                if self.word_wrap { "on" } else { "off" }
            )
        } else {
            self.last_action.clone()
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("menu_editor_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("menu_editor_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_save(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Save".to_string();
    }

    #[command]
    pub fn on_save_as(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Save As".to_string();
    }

    #[command]
    pub fn on_find(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Find".to_string();
    }

    #[command]
    pub fn on_replace(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Replace".to_string();
    }

    #[command]
    pub fn on_toggle_wrap(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.word_wrap = !self.word_wrap;
        self.last_action = format!("Word Wrap: {}", self.word_wrap);
    }
}

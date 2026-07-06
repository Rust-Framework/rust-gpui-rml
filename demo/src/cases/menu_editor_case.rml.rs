use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

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
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("check-side", "枚举", "勾选标记位置（Right/Left）"),
            ("menu-item checked", "布尔", "勾选状态绑定"),
            ("menu-item label", "字符串", "菜单项文案"),
            ("menu-item on-click", "事件", "点击回调"),
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
        r#"<!-- menu_editor_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- check-side="Right" 勾选标记显示在右侧 -->
    <dropdown-menu check-side="Right">
        <Button label="Edit" ghost="" />
        <menu-item label="Save" on-click={on_save} />
        <menu-item label="Save As" on-click={on_save_as} />
        <!-- menu-separator 分组 -->
        <menu-separator />
        <menu-item label="Find" icon="Search" on-click={on_find} />
        <menu-item label="Replace" icon="Replace" on-click={on_replace} />
        <menu-separator />
        <!-- checked={word_wrap} 绑定布尔字段 -->
        <menu-item label="Word Wrap" checked={word_wrap} on-click={on_toggle_wrap} />
    </dropdown-menu>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// menu_editor_case.rml.rs：后端状态 + computed + command handler
use gpui::SharedString;
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct MenuEditorCase {
    pub word_wrap: bool,
    pub last_action: String,
}

impl MenuEditorCase {
    // computed 显示当前状态（含 word_wrap 字段）
    #[computed]
    pub fn editor_status(&self) -> String {
        if self.last_action.is_empty() {
            format!("Word Wrap: {}", if self.word_wrap { "on" } else { "off" })
        } else {
            self.last_action.clone()
        }
    }

    #[command]
    pub fn on_save(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Save".to_string();
    }

    #[command]
    pub fn on_toggle_wrap(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.word_wrap = !self.word_wrap;
        self.last_action = format!("Word Wrap: {}", self.word_wrap);
    }
    // ... on_save_as / on_find / on_replace
}"#
            .to_string()
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

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

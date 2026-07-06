use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

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
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
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
        r#"<!-- menu_context_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- context-menu 包裹触发区域 -->
    <context-menu>
        <div>右键目标</div>
        <!-- menu-item label/icon/on-click -->
        <menu-item label="Open" icon="FolderOpen" on-click={on_open} />
        <menu-separator />
        <menu-item label="Copy" icon="BookOpen" on-click={on_copy} />
        <menu-item label="Cut" on-click={on_cut} />
        <menu-item label="Paste" on-click={on_paste} />
    </context-menu>

    <!-- 子菜单：menu-item 内嵌 menu-item -->
    <context-menu>
        <div>右键目标</div>
        <menu-item label="New">
            <menu-item label="File" icon="File" on-click={on_new_file} />
            <menu-item label="Folder" icon="Folder" on-click={on_new_folder} />
        </menu-item>
    </context-menu>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// menu_context_case.rml.rs：后端状态 + computed + command handler
use gpui::SharedString;
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct MenuContextCase {
    pub last_action: String,
}

impl MenuContextCase {
    // computed 展示当前状态
    #[computed]
    pub fn context_status(&self) -> String {
        if self.last_action.is_empty() {
            "空闲".to_string()
        } else {
            format!("上次操作: {}", self.last_action)
        }
    }

    // 每个 menu-item on-click 对应一个 #[command]
    #[command]
    pub fn on_open(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Open".to_string();
    }

    #[command]
    pub fn on_new_file(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "New File".to_string();
    }
    // ... 其他 on_copy / on_cut / on_paste / on_new_folder / on_delete
}"#
            .to_string()
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

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

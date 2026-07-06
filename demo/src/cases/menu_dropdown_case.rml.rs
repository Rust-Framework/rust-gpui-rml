use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

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
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
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
        r#"<!-- menu_dropdown_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- dropdown-menu 第一个子元素为触发器（Button） -->
    <dropdown-menu anchor="TopRight">
        <Button label="Options" ghost="" />
        <menu-item label="Custom Action" icon="Star" on-click={on_custom} />
        <menu-separator />
        <menu-item label="Standard Action" icon="Check" on-click={on_standard} />
        <menu-separator />
        <menu-item label="Exit" icon="Close" on-click={on_exit} />
    </dropdown-menu>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// menu_dropdown_case.rml.rs：后端状态 + computed + command handler
use gpui::SharedString;
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct MenuDropdownCase {
    pub last_action: String,
}

impl MenuDropdownCase {
    #[computed]
    pub fn dropdown_status(&self) -> String {
        if self.last_action.is_empty() { "空闲".into() } else { self.last_action.clone() }
    }

    #[command]
    pub fn on_custom(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Custom Action".to_string();
    }

    #[command]
    pub fn on_exit(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Exit".to_string();
    }
}"#
            .to_string()
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

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.menu.custom",
    kind = "case",
    group = "menu",
    order = 20,
)]
#[component]
#[derive(Default)]
pub struct MenuCustomCase {
    pub dark_mode: bool,
    pub last_action: String,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for MenuCustomCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.custom.title")
    }
}

impl ILifecycle for MenuCustomCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("menu-item header", "布尔标志", "分组标题（不可点击）"),
            ("menu-item label", "字符串", "菜单项文案"),
            ("menu-item on-click", "事件", "点击回调"),
            ("menu-item href", "URL", "外链跳转"),
            ("menu-item icon", "图标名", "菜单项图标"),
            ("menu-separator", "标签", "分组分隔线"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl MenuCustomCase {
    #[computed]
    pub fn dark_mode_label(&self) -> String {
        if self.dark_mode {
            rml_core::i18n::t_static("case.menu.on").to_string()
        } else {
            rml_core::i18n::t_static("case.menu.off").to_string()
        }
    }

    #[computed]
    pub fn custom_status(&self) -> String {
        self.last_action.clone()
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- menu_custom_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- header="" 作为分组标题（不可点击） -->
    <dropdown-menu>
        <Button label="Settings" ghost="" />
        <menu-item header="" label="Display" />
        <!-- checked={dark_mode} 绑定布尔字段 -->
        <menu-item label="Dark Mode" checked={dark_mode} on-click={on_toggle_dark} />
        <menu-separator />
        <!-- href 渲染为外链 -->
        <menu-item label="Help Center" href="https://rml.dev/help/" icon="Info" />
        <menu-item label="Sign Out" icon="ExternalLink" on-click={on_sign_out} />
    </dropdown-menu>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// menu_custom_case.rml.rs：后端状态 + computed + command handler
use gpui::SharedString;
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct MenuCustomCase {
    pub dark_mode: bool,
    pub last_action: String,
}

impl MenuCustomCase {
    // computed 根据 dark_mode 返回显示文案
    #[computed]
    pub fn dark_mode_label(&self) -> String {
        if self.dark_mode { "on".into() } else { "off".into() }
    }

    #[command]
    pub fn on_toggle_dark(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.dark_mode = !self.dark_mode;
        self.last_action = format!("Dark mode: {}", self.dark_mode);
    }

    #[command]
    pub fn on_sign_out(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Sign Out".to_string();
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_toggle_dark(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.dark_mode = !self.dark_mode;
        self.last_action = format!("Dark mode: {}", self.dark_mode);
    }

    #[command]
    pub fn on_sign_out(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Sign Out".to_string();
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

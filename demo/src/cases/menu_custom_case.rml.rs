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
    pub fn code_sample(&self) -> String {
        r#"<dropdown-menu>
    <Button label="Settings" ghost="" />
    <menu-item header="" label="Display" />
    <menu-item label="Dark Mode" on-click={on_toggle_dark} />
    <menu-separator />
    <menu-item label="Help" href="https://..." icon="Info" />
    <menu-item label="Sign Out" on-click={on_sign_out} />
</dropdown-menu>"#
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
}

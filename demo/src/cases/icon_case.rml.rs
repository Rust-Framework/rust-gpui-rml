use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{IconName, TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.icon",
    kind = "case",
    group = "components",
    order = 39,
)]
#[component]
#[derive(Default)]
pub struct IconCase {
    pub icon_index: u32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for IconCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.icon.title")
    }
}

impl ILifecycle for IconCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.icon_index = 0;
        let (cols, rows) = build_api_table(&[
            ("name", "IconName 枚举名", "图标名称（如 Settings/Bell/User），生成 Icon::new(IconName::Settings)"),
            ("path", "字符串", "自定义图标路径（如 icons/foo.svg），生成 Icon::empty().path(...)"),
            ("size", "枚举", "Sizable 尺寸：xsmall/small/medium/large"),
            ("text_color", "颜色", "来自 Styled trait，设置图标颜色"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl IconCase {
    #[computed]
    pub fn current_icon(&self) -> IconName {
        match self.icon_index % 3 {
            0 => IconName::Settings,
            1 => IconName::Bell,
            _ => IconName::User,
        }
    }

    #[computed]
    pub fn current_icon_name(&self) -> &'static str {
        match self.icon_index % 3 {
            0 => "Settings",
            1 => "Bell",
            _ => "User",
        }
    }

    #[command]
    pub fn on_rotate_icon(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.icon_index = self.icon_index.saturating_add(1);
        cx.notify();
    }
}

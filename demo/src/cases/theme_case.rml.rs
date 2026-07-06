use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "framework.theme",
    kind = "case",
    group = "framework",
    order = 46,
)]
#[component]
#[derive(Default)]
pub struct ThemeCase {
    pub theme_index: u8,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ThemeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.theme.title")
    }
}

impl ILifecycle for ThemeCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.theme_index = 0;
        let (cols, rows) = build_api_table(&[
            ("if={expr}", "指令", "根据主题索引条件渲染不同样式"),
            ("#[computed]", "方法", "派生当前主题标签"),
            ("#[command]", "方法", "按钮点击切换主题"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ThemeCase {
    #[computed]
    pub fn theme_label(&self) -> &'static str {
        match self.theme_index {
            0 => "默认（蓝）",
            1 => "主要（绿）",
            2 => "危险（红）",
            _ => "未知",
        }
    }

    #[command]
    pub fn on_theme_default(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.theme_index = 0;
    }

    #[command]
    pub fn on_theme_primary(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.theme_index = 1;
    }

    #[command]
    pub fn on_theme_danger(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.theme_index = 2;
    }
}

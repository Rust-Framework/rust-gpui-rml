use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.badge",
    kind = "case",
    group = "components",
    order = 22,
)]
#[component]
#[derive(Default)]
pub struct BadgeCase {
    pub count: i32,
    pub size_index: u8,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for BadgeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.badge.title")
    }
}

impl ILifecycle for BadgeCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.count = 5;
        let (cols, rows) = build_api_table(&[
            ("size", "small/medium/large", "尺寸(Sizable trait)"),
            ("子节点", "文本/数字", "徽标内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl BadgeCase {
    #[computed]
    pub fn badge_label(&self) -> String {
        if self.count > 99 {
            "99+".to_string()
        } else {
            self.count.to_string()
        }
    }

    #[computed]
    pub fn size_label(&self) -> &'static str {
        match self.size_index {
            0 => "small",
            1 => "medium",
            _ => "large",
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Badge>5</Badge>
<Badge size="small">5</Badge>
<Badge>99+</Badge>"#
            .to_string()
    }

    #[command]
    pub fn on_increment(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.count += 1;
    }

    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.size_index = (self.size_index + 1) % 3;
    }
}

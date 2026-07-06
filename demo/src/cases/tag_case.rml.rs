use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.tag",
    kind = "case",
    group = "components",
    order = 25,
)]
#[component]
#[derive(Default)]
pub struct TagCase {
    pub tag_text: String,
    pub variant_index: u8,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for TagCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tag.title")
    }
}

impl ILifecycle for TagCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.tag_text = "RML".into();
        let (cols, rows) = build_api_table(&[
            ("primary/secondary/danger/success/warning/info", "布尔标志", "变体颜色"),
            ("size", "small/medium", "尺寸"),
            ("子节点", "文本", "标签内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TagCase {
    #[computed]
    pub fn variant_label(&self) -> &'static str {
        match self.variant_index {
            0 => "default",
            1 => "primary",
            2 => "secondary",
            3 => "danger",
            4 => "success",
            5 => "warning",
            _ => "info",
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Tag>Default</Tag>
<Tag primary="">Primary</Tag>
<Tag danger="">Danger</Tag>
<Tag size="small">Small</Tag>"#
            .to_string()
    }

    #[command]
    pub fn on_cycle_variant(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.variant_index = (self.variant_index + 1) % 7;
    }
}

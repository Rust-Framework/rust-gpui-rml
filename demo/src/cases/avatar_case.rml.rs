use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.avatar",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct AvatarCase {
    pub name: String,
    pub size_index: u8,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for AvatarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.avatar.title")
    }
}

impl ILifecycle for AvatarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.name = "Jason Lee".into();
        let (cols, rows) = build_api_table(&[
            ("src", "URL 字符串", "图片地址"),
            ("name", "字符串", "取首字母显示"),
            ("placeholder", "图标名", "占位图标"),
            ("size", "small/medium/large", "尺寸变体"),
            ("AvatarGroup limit", "数字", "最大显示数量"),
            ("AvatarGroup ellipsis", "布尔标志", "溢出项显示 +N"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl AvatarCase {
    #[computed]
    pub fn size_label(&self) -> String {
        match self.size_index % 3 {
            0 => "small".to_string(),
            1 => "medium".to_string(),
            _ => "large".to_string(),
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Avatar src="https://..." size="large" />
<Avatar name="Jason Lee" />
<Avatar placeholder="Building2" />
<AvatarGroup limit="3" ellipsis="">
    <Avatar src="..." />
    <Avatar name="John" />
</AvatarGroup>"#
            .to_string()
    }

    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.size_index = (self.size_index + 1) % 3;
    }
}

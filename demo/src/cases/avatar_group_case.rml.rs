use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.avatar_group",
    kind = "case",
    group = "components",
    order = 29,
)]
#[component]
#[derive(Default)]
pub struct AvatarGroupCase {
    pub avatar_count: u8,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for AvatarGroupCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.avatar_group.title")
    }
}

impl ILifecycle for AvatarGroupCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.avatar_count = 3;
        let (cols, rows) = build_api_table(&[
            ("limit", "数字", "最大显示数量"),
            ("ellipsis", "布尔标志", "溢出显示 +N"),
            ("子节点", "Avatar[]", "头像列表"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl AvatarGroupCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<AvatarGroup limit="3" ellipsis="">
    <Avatar name="Alice" />
    <Avatar name="Bob" />
    <Avatar name="Charlie" />
</AvatarGroup>"#
            .to_string()
    }

    #[command]
    pub fn on_add_avatar(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        if self.avatar_count < 5 {
            self.avatar_count += 1;
        }
    }

    #[command]
    pub fn on_remove_avatar(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        if self.avatar_count > 1 {
            self.avatar_count -= 1;
        }
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

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
    pub group_api_columns: Vec<TableColumn>,
    pub group_api_rows: Vec<TableRow>,
    pub avatar_api_columns: Vec<TableColumn>,
    pub avatar_api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.avatar_count = 3;
        let (cols, rows) = build_api_table(&[
            ("limit", "数字", "最大显示数量"),
            ("ellipsis", "布尔标志", "溢出显示 +N"),
        ]);
        self.group_api_columns = cols;
        self.group_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("src", "字符串", "图片源 URL"),
            ("name", "字符串", "首字母 fallback"),
            ("placeholder", "图标名", "占位图标"),
        ]);
        self.avatar_api_columns = cols;
        self.avatar_api_rows = rows;
    }
}

impl AvatarGroupCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("avatar_group_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("avatar_group_case.rml.rs").to_string()
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

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{IconName, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

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
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.icon_index = 0;
        let (cols, rows) = build_api_table(&[
            ("name", "string", "内置图标名称，如 name=\"Settings\""),
            ("path", "string", "自定义图标路径，如 path=\"icons/foo.svg\""),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("text-color", "string", "图标颜色，如 text-color=\"var(--primary)\""),
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

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("icon_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("icon_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_rotate_icon(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.icon_index = self.icon_index.saturating_add(1);
    }
}

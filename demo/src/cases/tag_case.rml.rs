use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

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
    pub is_outline: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.tag_text = "RML".into();
        let (cols, rows) = build_api_table(&[
            ("primary / secondary / danger / success / warning / info", "布尔标志", "6 种 variant（构造器选择）"),
            ("outline", "布尔标志", "描边样式（透明背景 + 彩色边框/文字）"),
            ("size", "xsmall/small/medium/large", "尺寸（仅 Small/Medium 视觉区分）"),
            ("子节点", "文本/元素", "标签内容"),
            ("on-click", "事件", "点击回调"),
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
    pub fn outline_label(&self) -> &'static str {
        if self.is_outline { "outline" } else { "filled" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("tag_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("tag_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_cycle_variant(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.variant_index = (self.variant_index + 1) % 7;
    }

    #[command]
    pub fn on_toggle_outline(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_outline = !self.is_outline;
    }

    #[command]
    pub fn on_cycle_text(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.tag_text = match self.tag_text.as_str() {
            "RML" => "Rust".into(),
            "Rust" => "GPUI".into(),
            _ => "RML".into(),
        };
    }
}

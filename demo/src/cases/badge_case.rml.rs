use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

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
    pub count: usize,
    pub max_val: usize,
    pub is_dot: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.count = 5;
        self.max_val = 9;
        let (cols, rows) = build_api_table(&[
            ("count", "number / binding", "数字徽标计数（0 时隐藏）"),
            ("max", "number / binding", "最大显示数，超出显示 N+（默认 99）"),
            ("dot", "bool", "小红点模式"),
            ("icon", "string", "图标徽标，如 icon=\"Bell\""),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("（子节点）", "slot", "被徽标标记的内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl BadgeCase {
    #[computed]
    pub fn count_display(&self) -> String {
        if self.count > self.max_val {
            format!("{}+", self.max_val)
        } else {
            self.count.to_string()
        }
    }

    #[computed]
    pub fn variant_label(&self) -> &'static str {
        if self.is_dot { "Dot" } else { "Number" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("badge_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("badge_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_increment(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count += 1;
    }

    #[command]
    pub fn on_increment_10(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count += 10;
    }

    #[command]
    pub fn on_reset(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count = 0;
    }

    #[command]
    pub fn on_toggle_dot(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_dot = !self.is_dot;
    }
}

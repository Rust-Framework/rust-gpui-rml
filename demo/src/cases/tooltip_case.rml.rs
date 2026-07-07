use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.tooltip",
    kind = "case",
    group = "components",
    order = 61,
)]
#[component]
#[derive(Default)]
pub struct TooltipCase {
    pub tooltip_index: u8,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TooltipCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tooltip.title")
    }
}

impl ILifecycle for TooltipCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.tooltip_index = 0;
        let (cols, rows) = build_api_table(&[
            ("tooltip", "字符串 / 绑定", "悬浮提示文本，映射到 .tooltip(text)"),
            ("支持组件", "白名单", "Button / IconButton / DropdownButton / Toggle / Checkbox / Clipboard / Radio / Switch"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TooltipCase {
    #[computed]
    pub fn dynamic_tooltip(&self) -> SharedString {
        match self.tooltip_index % 3 {
            0 => "提示 A：保存当前文档".into(),
            1 => "提示 B：导出为 PDF".into(),
            _ => "提示 C：分享链接".into(),
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("tooltip_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("tooltip_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_cycle_tooltip(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.tooltip_index = self.tooltip_index.saturating_add(1);
    }
}

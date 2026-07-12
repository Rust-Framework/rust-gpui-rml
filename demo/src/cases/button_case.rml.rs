use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.button",
    kind = "case",
    group = "components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub basic_clicks: i32,
    pub is_disabled: bool,
    pub is_selected: bool,
    pub is_loading: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ButtonCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.button.title")
    }
}

impl ILifecycle for ButtonCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("label", "string", "按钮文字"),
            ("on-click", "event", "点击回调（参数：&ClickEvent）"),
            ("primary / secondary / danger / success / warning / info / ghost / link / text", "bool", "9 种 variant，默认 secondary"),
            ("size", "xsmall / small / medium / large", "尺寸，默认 medium"),
            ("icon", "string", "图标名称（PascalCase），如 icon=\"Play\"、icon=\"Delete\""),
            ("disabled", "bool / binding", "禁用，默认 false"),
            ("loading", "bool / binding", "加载中，默认 false"),
            ("selected", "bool / binding", "选中态，默认 false"),
            ("compact", "bool", "紧凑内边距"),
            ("tooltip", "string", "悬浮提示文本"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ButtonCase {
    #[computed]
    pub fn disabled_status_text(&self) -> String {
        if self.is_disabled { "禁用".into() } else { "可用".into() }
    }

    #[computed]
    pub fn selected_status_text(&self) -> String {
        if self.is_selected { "选中".into() } else { "未选中".into() }
    }

    #[computed]
    pub fn loading_status_text(&self) -> String {
        if self.is_loading { "加载中".into() } else { "空闲".into() }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("button_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("button_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_basic_click(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.basic_clicks += 1;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_toggle_selected(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_selected = !self.is_selected;
    }

    #[command]
    pub fn on_toggle_loading(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_loading = !self.is_loading;
    }
}

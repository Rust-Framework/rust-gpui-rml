use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{CalendarState, Date, Size, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.calendar",
    kind = "case",
    group = "components",
    order = 76,
)]
#[component]
#[derive(Default)]
pub struct CalendarCase {
    /// Section 1：基础用法 + on_select 事件
    pub basic_calendar: ElementRef<CalendarState>,
    pub selected_date: String,

    /// Section 2：尺寸 size
    pub sized_calendar: ElementRef<CalendarState>,
    pub current_size: u8,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for CalendarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.calendar.title")
    }
}

impl ILifecycle for CalendarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段，如 ref=\"basic_calendar\""),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("on-select", "event", "选择日期时回调，参数为所选日期"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CalendarCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("calendar_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("calendar_case.rml.rs").to_string()
    }

    #[computed]
    pub fn size_label(&self) -> &'static str {
        match self.current_size {
            0 => "xsmall",
            1 => "small",
            2 => "medium",
            _ => "large",
        }
    }

    #[computed]
    pub fn size_value(&self) -> Size {
        match self.current_size {
            0 => Size::XSmall,
            1 => Size::Small,
            2 => Size::Medium,
            _ => Size::Large,
        }
    }

    /// Section 1：on_select 事件，参数为 Date
    #[command]
    pub fn on_date_select(&mut self, date: Date, _cx: &mut Context<Self>) {
        self.selected_date = format!("{}", date);
    }

    /// Section 2：循环切换 size
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_size = (self.current_size + 1) % 4;
    }
}

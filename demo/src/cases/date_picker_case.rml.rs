use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{DatePickerState, Date, Size, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.date_picker",
    kind = "case",
    group = "components",
    order = 77,
)]
#[component]
#[derive(Default)]
pub struct DatePickerCase {
    /// Section 1：基础用法 + on_change 事件
    pub basic_picker: ElementRef<DatePickerState>,
    pub selected_date: String,

    /// Section 2：placeholder + cleanable
    pub cleanable_picker: ElementRef<DatePickerState>,
    pub cleanable_date: String,

    /// Section 3：appearance=false 最小样式
    pub minimal_picker: ElementRef<DatePickerState>,
    pub minimal_date: String,

    /// Section 4：尺寸 size
    pub sized_picker: ElementRef<DatePickerState>,
    pub current_size: u8,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for DatePickerCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.date_picker.title")
    }
}

impl ILifecycle for DatePickerCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("ref", "字符串（指令）", "元素引用名，绑定到 ElementRef<DatePickerState> 字段"),
            ("placeholder", "字符串", "占位文本（走通用 static setter）"),
            ("cleanable", "布尔属性", "启用清除按钮（默认 false）"),
            ("appearance", "true/false", "是否显示边框背景（默认 true，设 false 移除）"),
            ("number_of_months", "usize", "日历显示月份数（默认 2）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait 通用属性）"),
            ("on_change", "事件", "日期变化回调（参数：Date；通过 cx.subscribe 订阅 DatePickerEvent::Change）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl DatePickerCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("date_picker_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("date_picker_case.rml.rs").to_string()
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

    /// Section 1：on_change 事件，参数为 Date
    #[command]
    pub fn on_date_change(&mut self, date: Date, _cx: &mut Context<Self>) {
        self.selected_date = format!("{}", date);
    }

    /// Section 2：cleanable 日期变化
    #[command]
    pub fn on_cleanable_change(&mut self, date: Date, _cx: &mut Context<Self>) {
        self.cleanable_date = format!("{}", date);
    }

    /// Section 3：最小样式日期变化
    #[command]
    pub fn on_minimal_change(&mut self, date: Date, _cx: &mut Context<Self>) {
        self.minimal_date = format!("{}", date);
    }

    /// Section 4：循环切换 size
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_size = (self.current_size + 1) % 4;
    }
}

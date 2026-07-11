use gpui::SharedString;
use gpui::Hsla;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{ColorPickerState, Size, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.color_picker",
    kind = "case",
    group = "components",
    order = 75,
)]
#[component]
#[derive(Default)]
pub struct ColorPickerCase {
    /// Section 1：基础用法 + on_change 事件
    pub basic_picker: ElementRef<ColorPickerState>,
    pub current_color: String,

    /// Section 2：label 属性
    pub labeled_picker: ElementRef<ColorPickerState>,

    /// Section 3：icon 属性
    pub icon_picker: ElementRef<ColorPickerState>,

    /// Section 4：尺寸 size
    pub sized_picker: ElementRef<ColorPickerState>,
    pub current_size: u8,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ColorPickerCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.color_picker.title")
    }
}

impl ILifecycle for ColorPickerCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("ref", "字符串（指令）", "元素引用名，绑定到 ElementRef<ColorPickerState> 字段"),
            ("label", "字符串", "标签文本（走通用 static setter）"),
            ("icon", "IconName", "图标名称（如 icon=\"Palette\"）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait 通用属性）"),
            ("on_change", "事件", "颜色变化回调（参数：Option<Hsla>；通过 cx.subscribe 订阅 ColorPickerEvent::Change）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ColorPickerCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("color_picker_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("color_picker_case.rml.rs").to_string()
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

    /// Section 1：on_change 事件，参数为 Option<Hsla>
    #[command]
    pub fn on_color_change(&mut self, color: Option<Hsla>, _cx: &mut Context<Self>) {
        self.current_color = match color {
            Some(c) => format!("hsla({}, {}, {}, {})", c.h, c.s, c.l, c.a),
            None => "None".to_string(),
        };
    }

    /// Section 4：循环切换 size
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_size = (self.current_size + 1) % 4;
    }
}

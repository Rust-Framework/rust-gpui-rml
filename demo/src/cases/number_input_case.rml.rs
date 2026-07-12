use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{InputState, Size, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.number_input",
    kind = "case",
    group = "components",
    order = 73,
)]
#[component]
#[derive(Default)]
pub struct NumberInputCase {
    /// Section 1：基础用法 + ref 指令
    pub basic_input: ElementRef<InputState>,

    /// Section 2：placeholder + on_change 事件
    pub change_input: ElementRef<InputState>,
    pub current_value: String,

    /// Section 3：value 双向绑定（InputStateBridge）
    pub bound_input: ElementRef<InputState>,
    pub bound_value: String,

    /// Section 4：disabled 禁用
    pub disabled_input: ElementRef<InputState>,
    pub is_disabled: bool,

    /// Section 5：尺寸 size
    pub sized_input: ElementRef<InputState>,
    pub current_size: u8,

    /// Section 6：appearance 无边框
    pub bordered_input: ElementRef<InputState>,
    pub borderless_input: ElementRef<InputState>,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for NumberInputCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.number_input.title")
    }
}

impl ILifecycle for NumberInputCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段，如 ref=\"basic_input\""),
            ("value", "binding", "双向绑定到 ViewModel 字段，如 value={count}"),
            ("placeholder", "string", "占位文本"),
            ("appearance", "bool", "是否显示边框和背景（默认 true，appearance=\"false\" 移除）"),
            ("disabled", "bool / binding", "禁用状态"),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("on-change", "event", "内容变化时回调"),
            ("on-enter", "event", "按回车时回调"),
            ("on-focus", "event", "获得焦点时回调"),
            ("on-blur", "event", "失去焦点时回调"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl NumberInputCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("number_input_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("number_input_case.rml.rs").to_string()
    }

    #[computed]
    pub fn disabled_status_text(&self) -> &'static str {
        if self.is_disabled {
            "已禁用"
        } else {
            "未禁用"
        }
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

    /// Section 2：on_change 事件，参数为 &Entity<InputState>
    #[command]
    pub fn on_num_change(&mut self, entity: &gpui::Entity<InputState>, _cx: &mut Context<Self>) {
        self.current_value = entity.read(_cx).value().to_string();
    }

    /// Section 4：切换 disabled 状态
    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    /// Section 5：循环切换 size
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_size = (self.current_size + 1) % 4;
    }
}

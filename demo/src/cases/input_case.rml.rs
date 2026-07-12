use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{Size, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.input",
    kind = "case",
    group = "components",
    order = 35,
)]
#[component]
#[derive(Default)]
pub struct InputCase {
    /// Section 1：基础用法 + ref 指令
    pub basic_input: ElementRef<rml_ui::InputState>,

    /// Section 2：placeholder（ref 路径直接支持）
    pub placeholder_input: ElementRef<rml_ui::InputState>,

    /// Section 3：default_value + masked
    pub masked_input: ElementRef<rml_ui::InputState>,

    /// Section 4：disabled 禁用
    pub disabled_input: ElementRef<rml_ui::InputState>,
    pub is_disabled: bool,

    /// Section 5：尺寸 size
    pub sized_input: ElementRef<rml_ui::InputState>,
    pub current_size: u8,

    /// Section 6：selected 选中态
    pub selected_input: ElementRef<rml_ui::InputState>,
    pub is_selected: bool,

    /// Section 7：多 Input 组合（表单布局）
    pub form_name_input: ElementRef<rml_ui::InputState>,
    pub form_email_input: ElementRef<rml_ui::InputState>,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for InputCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.input.title")
    }
}

impl ILifecycle for InputCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段，如 ref=\"basic_input\""),
            ("value", "binding", "双向绑定到 ViewModel 字段，如 value={username}"),
            ("placeholder", "string / binding", "占位文本，如 placeholder=\"用户名\""),
            ("default-value", "string", "初始值，如 default-value=\"hello\""),
            ("masked", "bool", "密码遮罩模式"),
            ("disabled", "bool / binding", "禁用状态"),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("selected", "bool / binding", "选中态"),
            ("on-change", "event", "内容变化时回调"),
            ("on-enter", "event", "按回车时回调"),
            ("on-focus", "event", "获得焦点时回调"),
            ("on-blur", "event", "失去焦点时回调"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl InputCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("input_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("input_case.rml.rs").to_string()
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
    pub fn selected_status_text(&self) -> &'static str {
        if self.is_selected {
            "已选中"
        } else {
            "未选中"
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

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_size = (self.current_size + 1) % 4;
    }

    #[command]
    pub fn on_toggle_selected(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_selected = !self.is_selected;
    }
}

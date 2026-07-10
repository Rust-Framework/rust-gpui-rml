use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{InputState, Size, TableColumn, TableRow};

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
    /// `<Input ref="basic_input" />` 惰性创建 Entity<InputState>，
    /// 首次 render 后由 __rml_populate_refs 注入到此字段。
    pub basic_input: ElementRef<InputState>,

    /// Section 2：placeholder 设置时机
    /// Pattern B：在 on_loaded 中创建 InputState Entity 并配置 placeholder，
    /// .rml 中用 `<Input />`（无 ref）通过 state_field 路径取用。
    /// 字段名必须为 input_state（tags.rs 中 Input 的 state_field 硬编码）。
    /// 此模式适合需要在创建时配置 InputState builder 的场景（placeholder/default_value/masked 等）。
    pub input_state: Option<gpui::Entity<InputState>>,

    /// Section 3：disabled 禁用
    /// Pattern A：ref + ElementRef，通过 RML 组件属性 `disabled={is_disabled}` 切换。
    pub disabled_input: ElementRef<InputState>,
    pub is_disabled: bool,

    /// Section 4：尺寸 size
    /// 通过 RML 组件属性 `size={size_value}` 切换（Sizable trait 通用属性）。
    /// size_value computed 返回 rml_ui::Size 枚举值，供 with_size(impl Into<Size>) 使用。
    pub sized_input: ElementRef<InputState>,
    pub current_size: u8,

    /// Section 5：selected 选中态
    /// 通过 RML 组件属性 `selected={is_selected}` 切换（Selectable trait 通用属性）。
    pub selected_input: ElementRef<InputState>,
    pub is_selected: bool,

    /// Section 6：多 Input 组合（表单布局）
    /// 多个 ref 字段在 flex 布局中并排，演示真实表单场景。
    pub form_name_input: ElementRef<InputState>,
    pub form_email_input: ElementRef<InputState>,

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

        // Section 2：在 on_loaded 中创建 InputState Entity 并配置 placeholder。
        // Pattern B 的核心：state_ctor 是 `InputState::new(w, c)`，无法在 ref 路径注入
        // placeholder 等 builder 参数；改在 on_loaded 中手动 cx.new 创建，配合
        // `<Input />`（无 ref）通过 self.input_state.as_ref().expect(...) 取用。
        self.input_state = Some(cx.new(|cx| {
            InputState::new(_window, cx).placeholder("请输入用户名（on_loaded 中配置）")
        }));

        let (cols, rows) = build_api_table(&[
            ("ref", "字符串（指令）", "元素引用名，绑定到 ElementRef<InputState> 字段（PascalCase <Input>）"),
            ("disabled", "布尔/绑定", "禁用状态（Input 组件属性）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait 通用属性，绑定需返回 Size 枚举）"),
            ("selected", "布尔/绑定", "选中态（Selectable trait）"),
            ("on_change", "事件", "内容变化回调（参数：&InputState；通过 cx.subscribe 订阅 InputEvent::Change）"),
            ("on_enter", "事件", "回车按下回调（InputEvent::PressEnter）"),
            ("on_focus", "事件", "获得焦点回调（InputEvent::Focus）"),
            ("on_blur", "事件", "失去焦点回调（InputEvent::Blur）"),
            ("value", "绑定属性", "双向绑定到 pub 字段（仅小写 <input> 标签支持，见 two_way_case 演示）"),
            ("placeholder", "字符串", "占位文本（仅小写 <input value={...}> 支持；PascalCase Input 需在 on_loaded 中通过 InputState builder 配置）"),
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

    /// 返回 rml_ui::Size 枚举值供 size={size_value} 绑定使用。
    /// size 绑定走 component_bind_setter，生成 .with_size(self.size_value())，
    /// with_size 接收 impl Into<Size>，Size 本身实现 Into<Size>（identity）。
    #[computed]
    pub fn size_value(&self) -> Size {
        match self.current_size {
            0 => Size::XSmall,
            1 => Size::Small,
            2 => Size::Medium,
            _ => Size::Large,
        }
    }

    /// Section 3：切换 disabled 状态
    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    /// Section 4：循环切换 size（xsmall → small → medium → large → xsmall）
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_size = (self.current_size + 1) % 4;
    }

    /// Section 5：切换 selected 状态
    #[command]
    pub fn on_toggle_selected(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_selected = !self.is_selected;
    }
}

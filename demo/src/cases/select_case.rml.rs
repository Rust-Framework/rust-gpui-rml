use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{SearchableVec, SelectState, Size, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.select",
    kind = "case",
    group = "components",
    order = 78,
)]
#[component]
pub struct SelectCase {
    /// Section 1：基础用法 + on_change 事件
    pub basic_select: ElementRef<SelectState<SearchableVec<SharedString>>>,
    pub basic_items: SearchableVec<SharedString>,
    pub selected_value: String,

    /// Section 2：placeholder + cleanable
    pub cleanable_select: ElementRef<SelectState<SearchableVec<SharedString>>>,
    pub city_items: SearchableVec<SharedString>,
    pub selected_city: String,

    /// Section 3：appearance=false + menu_width/menu_max_h
    pub minimal_select: ElementRef<SelectState<SearchableVec<SharedString>>>,
    pub lang_items: SearchableVec<SharedString>,
    pub selected_lang: String,

    /// Section 4：尺寸 size
    pub sized_select: ElementRef<SelectState<SearchableVec<SharedString>>>,
    pub current_size: u8,

    /// Section 5：value 双向绑定
    pub bound_fruit: String,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl Default for SelectCase {
    fn default() -> Self {
        Self {
            basic_select: Default::default(),
            basic_items: SearchableVec::new(Vec::<SharedString>::new()),
            selected_value: Default::default(),
            cleanable_select: Default::default(),
            city_items: SearchableVec::new(Vec::<SharedString>::new()),
            selected_city: Default::default(),
            minimal_select: Default::default(),
            lang_items: SearchableVec::new(Vec::<SharedString>::new()),
            selected_lang: Default::default(),
            sized_select: Default::default(),
            current_size: Default::default(),
            bound_fruit: "苹果".to_string(),
            api_columns: Default::default(),
            api_rows: Default::default(),
            case_doc_page: Default::default(),
            __rml_state: Default::default(),
        }
    }
}

impl IContribution for SelectCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.select.title")
    }
}

impl ILifecycle for SelectCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        self.basic_items = SearchableVec::new(vec![
            SharedString::from("苹果"),
            SharedString::from("香蕉"),
            SharedString::from("橙子"),
            SharedString::from("葡萄"),
            SharedString::from("西瓜"),
        ]);
        self.city_items = SearchableVec::new(vec![
            SharedString::from("北京"),
            SharedString::from("上海"),
            SharedString::from("广州"),
            SharedString::from("深圳"),
            SharedString::from("杭州"),
        ]);
        self.lang_items = SearchableVec::new(vec![
            SharedString::from("Rust"),
            SharedString::from("TypeScript"),
            SharedString::from("Python"),
            SharedString::from("Go"),
            SharedString::from("C++"),
        ]);

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段（与 items 配合使用）"),
            ("items", "binding", "选项数据源，如 items={fruit_list}"),
            ("value", "binding", "双向绑定到 ViewModel 字符串字段，如 value={selected}"),
            ("placeholder", "string", "占位文本"),
            ("cleanable", "bool", "启用清除按钮（默认 false）"),
            ("appearance", "bool", "是否显示边框背景（默认 true，设 false 移除）"),
            ("menu-width", "string", "下拉菜单宽度，如 menu-width=\"240px\""),
            ("menu-max-h", "string", "下拉菜单最大高度，如 menu-max-h=\"200px\""),
            ("size", "string", "尺寸：xsmall | small | medium | large"),
            ("on-change", "event", "选择确认时回调，参数为选中值或空（清除后）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SelectCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("select_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("select_case.rml.rs").to_string()
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

    /// Section 1：on_change 事件，参数为 Option<SharedString>
    #[command]
    pub fn on_basic_change(&mut self, value: Option<SharedString>, _cx: &mut Context<Self>) {
        self.selected_value = value
            .map(|v| v.to_string())
            .unwrap_or_else(|| "（已清除）".to_string());
    }

    /// Section 2：cleanable 城市选择
    #[command]
    pub fn on_city_change(&mut self, value: Option<SharedString>, _cx: &mut Context<Self>) {
        self.selected_city = value
            .map(|v| v.to_string())
            .unwrap_or_else(|| "（已清除）".to_string());
    }

    /// Section 3：语言选择
    #[command]
    pub fn on_lang_change(&mut self, value: Option<SharedString>, _cx: &mut Context<Self>) {
        self.selected_lang = value
            .map(|v| v.to_string())
            .unwrap_or_else(|| "（已清除）".to_string());
    }

    /// Section 4：循环切换 size
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_size = (self.current_size + 1) % 4;
    }
}

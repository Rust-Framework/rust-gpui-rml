use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{ComboboxState, SearchableVec, Size, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.combobox",
    kind = "case",
    group = "components",
    order = 79,
)]
#[component]
pub struct ComboboxCase {
    /// Section 1：基础用法 + on_change 事件
    pub basic_combobox: ElementRef<ComboboxState<SearchableVec<SharedString>>>,
    pub basic_items: SearchableVec<SharedString>,
    pub selected_values: String,

    /// Section 2：placeholder + cleanable + search_placeholder
    pub multi_combobox: ElementRef<ComboboxState<SearchableVec<SharedString>>>,
    pub tag_items: SearchableVec<SharedString>,
    pub selected_tags: String,

    /// Section 3：appearance=false + menu_width/menu_max_h
    pub minimal_combobox: ElementRef<ComboboxState<SearchableVec<SharedString>>>,
    pub lang_items: SearchableVec<SharedString>,
    pub selected_langs: String,

    /// Section 4：尺寸 size
    pub sized_combobox: ElementRef<ComboboxState<SearchableVec<SharedString>>>,
    pub current_size: u8,

    /// Section 5：value 双向绑定
    pub bound_tags: Vec<String>,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl Default for ComboboxCase {
    fn default() -> Self {
        Self {
            basic_combobox: Default::default(),
            basic_items: SearchableVec::new(Vec::<SharedString>::new()),
            selected_values: Default::default(),
            multi_combobox: Default::default(),
            tag_items: SearchableVec::new(Vec::<SharedString>::new()),
            selected_tags: Default::default(),
            minimal_combobox: Default::default(),
            lang_items: SearchableVec::new(Vec::<SharedString>::new()),
            selected_langs: Default::default(),
            sized_combobox: Default::default(),
            current_size: Default::default(),
            bound_tags: vec!["Rust".to_string(), "RML".to_string()],
            api_columns: Default::default(),
            api_rows: Default::default(),
            case_doc_page: Default::default(),
            __rml_state: Default::default(),
        }
    }
}

impl IContribution for ComboboxCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.combobox.title")
    }
}

impl ILifecycle for ComboboxCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        self.basic_items = SearchableVec::new(vec![
            SharedString::from("苹果"),
            SharedString::from("香蕉"),
            SharedString::from("橙子"),
            SharedString::from("葡萄"),
            SharedString::from("西瓜"),
        ]);
        self.tag_items = SearchableVec::new(vec![
            SharedString::from("Rust"),
            SharedString::from("GPUI"),
            SharedString::from("RML"),
            SharedString::from("MVVM"),
            SharedString::from("声明式"),
            SharedString::from("组件化"),
        ]);
        self.lang_items = SearchableVec::new(vec![
            SharedString::from("Rust"),
            SharedString::from("TypeScript"),
            SharedString::from("Python"),
            SharedString::from("Go"),
            SharedString::from("C++"),
        ]);

        let (cols, rows) = build_api_table(&[
            ("ref", "字符串（指令）", "元素引用名，绑定到 ElementRef<ComboboxState<SearchableVec<SharedString>>> 字段（必填）"),
            ("items", "绑定表达式", "SearchableVec<SharedString> 委托数据源，通过 items={field} 绑定（value 双向绑定时必填）"),
            ("value", "绑定属性", "双向绑定到 pub Vec<String> 字段（StateBridge → set_selected_indices / ComboboxEvent::Change）"),
            ("placeholder", "字符串", "占位文本（走通用 static setter）"),
            ("cleanable", "布尔属性", "启用清除按钮（默认 false）"),
            ("appearance", "true/false", "是否显示边框背景（默认 true，设 false 移除）"),
            ("menu-width", "像素值（如 280px）", "下拉菜单宽度"),
            ("menu-max-h", "像素值（如 180px）", "下拉菜单最大高度"),
            ("search-placeholder", "字符串", "搜索框占位文本"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait 通用属性）"),
            ("on_change", "事件", "选择变化回调（参数：Vec<SharedString>；通过 cx.subscribe 订阅 ComboboxEvent::Change）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ComboboxCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("combobox_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("combobox_case.rml.rs").to_string()
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

    #[computed]
    pub fn bound_tags_label(&self) -> String {
        Self::format_values(
            &self
                .bound_tags
                .iter()
                .map(|s| SharedString::from(s.as_str()))
                .collect::<Vec<_>>(),
        )
    }

    fn format_values(values: &[SharedString]) -> String {
        if values.is_empty() {
            "（无）".to_string()
        } else {
            values
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    /// Section 1：on_change 事件，参数为 Vec<SharedString>
    #[command]
    pub fn on_basic_change(&mut self, values: Vec<SharedString>, _cx: &mut Context<Self>) {
        self.selected_values = Self::format_values(&values);
    }

    /// Section 2：标签多选
    #[command]
    pub fn on_tag_change(&mut self, values: Vec<SharedString>, _cx: &mut Context<Self>) {
        self.selected_tags = Self::format_values(&values);
    }

    /// Section 3：语言选择
    #[command]
    pub fn on_lang_change(&mut self, values: Vec<SharedString>, _cx: &mut Context<Self>) {
        self.selected_langs = Self::format_values(&values);
    }

    /// Section 4：循环切换 size
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_size = (self.current_size + 1) % 4;
    }
}

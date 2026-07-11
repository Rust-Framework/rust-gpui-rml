use gpui::SharedString;
use rml::prelude::*;
use rml_core::element_ref::ElementRef;
use rml_core::i18n::t_static;
use rml_ui::{SliderState, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.slider",
    kind = "case",
    group = "components",
    order = 37,
)]
#[component]
#[derive(Default)]
pub struct SliderCase {
    /// 基础滑块：min=0 max=100 step=1 default=50
    pub slider_state: ElementRef<SliderState>,
    /// 禁用滑块：min=0 max=100 default=30
    pub disabled_state: ElementRef<SliderState>,
    /// 范围滑块：min=0 max=100 step=5 default=(20, 80)
    pub range_state: ElementRef<SliderState>,
    /// 范围滑块默认值，通过 default-value={range_default} 绑定
    pub range_default: (f32, f32),

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for SliderCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.slider.title")
    }
}

impl ILifecycle for SliderCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        self.range_default = (20.0, 80.0);

        let (cols, rows) = build_api_table(&[
            ("ref", "字符串", "SliderState 元素引用名（配合 ElementRef<SliderState> 字段）"),
            ("min", "f32（Static）", "最小值（如 min=\"0\"）"),
            ("max", "f32（Static）", "最大值（如 max=\"100\"）"),
            ("step", "f32（Static）", "步长（如 step=\"1\"）"),
            ("default-value", "f32 或 (f32, f32) 绑定", "初始值：Static 单值（default-value=\"50\"）或 Bind 元组（default-value={range_default}）"),
            ("disabled", "布尔/绑定", "禁用滑块交互"),
            ("on-change", "事件", "值变化回调（参数：SliderValue；通过 cx.subscribe 订阅 SliderEvent::Change）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SliderCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("slider_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("slider_case.rml.rs").to_string()
    }
}

use gpui::SharedString;
use rml::prelude::*;
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
    pub slider_state: Option<gpui::Entity<SliderState>>,
    pub disabled_state: Option<gpui::Entity<SliderState>>,
    pub range_state: Option<gpui::Entity<SliderState>>,
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

        self.slider_state = Some(cx.new(|_cx| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(50.0)
        }));
        self.disabled_state = Some(cx.new(|_cx| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .default_value(30.0)
        }));
        self.range_state = Some(cx.new(|_cx| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(5.0)
                .default_value((20.0, 80.0))
        }));
        let (cols, rows) = build_api_table(&[
            ("ref", "字符串", "SliderState Entity 引用名（on_loaded 中初始化）"),
            ("disabled", "布尔/绑定", "禁用滑块交互"),
            ("SliderState::min/max", "f32", "范围（on_loaded 中设置）"),
            ("SliderState::step", "f32", "步长"),
            ("SliderState::default_value", "f32 / (f32, f32)", "初始值（单值或范围）"),
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

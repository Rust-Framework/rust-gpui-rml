use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{SliderState, TableColumn, TableRow};

use crate::cases::common::build_api_table;

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
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
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
        r#"<!-- slider_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础滑块：ref 引用 on_loaded 中初始化的 SliderState -->
    <Slider ref="slider_state" />

    <!-- 禁用滑块：disabled={true} -->
    <Slider ref="disabled_state" disabled={true} />

    <!-- 范围滑块：default_value((20.0, 80.0)) 在 on_loaded 中设置 -->
    <Slider ref="range_state" />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// slider_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;
use rml_ui::SliderState;

#[component]
#[derive(Default)]
pub struct SliderCase {
    pub slider_state: Option<gpui::Entity<SliderState>>,
    pub disabled_state: Option<gpui::Entity<SliderState>>,
    pub range_state: Option<gpui::Entity<SliderState>>,
}

impl ILifecycle for SliderCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, cx: &mut Context<Self>) {
        // 基础滑块：min/max/step/default_value
        self.slider_state = Some(cx.new(|_cx| {
            SliderState::new()
                .min(0.0).max(100.0).step(1.0)
                .default_value(50.0)
        }));

        // 禁用滑块
        self.disabled_state = Some(cx.new(|_cx| {
            SliderState::new()
                .min(0.0).max(100.0)
                .default_value(30.0)
        }));

        // 范围滑块：default_value 接受 (f32, f32) 元组
        self.range_state = Some(cx.new(|_cx| {
            SliderState::new()
                .min(0.0).max(100.0).step(5.0)
                .default_value((20.0, 80.0))
        }));
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

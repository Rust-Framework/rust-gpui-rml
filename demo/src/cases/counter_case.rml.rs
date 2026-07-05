use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "binding.counter",
    kind = "case",
    group = "binding",
    order = 1,
)]
#[component]
#[derive(Default)]
pub struct CounterCase {
    pub count: i32,
    pub step: i32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for CounterCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.counter.title")
    }
}

impl ILifecycle for CounterCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("pub 字段", "i32/String/bool", "observable 状态"),
            ("#[computed]", "方法", "缓存计算属性"),
            ("#[command]", "方法", "事件处理 + 状态更新"),
            ("on-click", "事件", "按钮点击回调"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CounterCase {
    #[computed]
    pub fn counter_text(&self) -> String {
        format!("点击次数：{}", self.count)
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Button label="点击 +1" onclick={on_click} />
<p>{counter_text}</p>"#.to_string()
    }

    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += self.step.max(1);
    }

    #[command]
    pub fn on_inc_step(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.step += 1;
    }

    #[command]
    pub fn on_reset(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count = 0;
        self.step = 0;
    }
}

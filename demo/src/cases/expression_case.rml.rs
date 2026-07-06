use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "framework.expression",
    kind = "case",
    group = "framework",
    order = 41,
)]
#[component]
#[derive(Default)]
pub struct ExpressionCase {
    pub a: i32,
    pub b: i32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ExpressionCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.expression.title")
    }
}

impl ILifecycle for ExpressionCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.a = 12;
        self.b = 8;
        let (cols, rows) = build_api_table(&[
            ("{expr}", "表达式", "文本插值中支持任意 Rust 表达式"),
            ("#[computed]", "方法", "依赖字段自动重算的派生值"),
            ("属性={expr}", "绑定", "组件属性绑定表达式"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ExpressionCase {
    #[computed]
    pub fn sum(&self) -> i32 {
        self.a + self.b
    }

    #[computed]
    pub fn product(&self) -> i32 {
        self.a * self.b
    }

    #[computed]
    pub fn quotient(&self) -> String {
        if self.b == 0 {
            "除零错误".to_string()
        } else {
            format!("{:.2}", self.a as f64 / self.b as f64)
        }
    }

    #[command]
    pub fn on_increase_a(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.a += 1;
        cx.notify();
    }

    #[command]
    pub fn on_increase_b(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.b += 1;
        cx.notify();
    }
}

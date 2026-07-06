use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.progress",
    kind = "case",
    group = "components",
    order = 26,
)]
#[component]
#[derive(Default)]
pub struct ProgressCase {
    pub current: f32,
    pub loading: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ProgressCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.progress.title")
    }
}

impl ILifecycle for ProgressCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.current = 60.0;
        let (cols, rows) = build_api_table(&[
            ("value", "f32", "进度值 0-100"),
            ("loading", "布尔标志", "加载中状态"),
            ("size", "small/medium/large", "尺寸"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
        cx.notify();
    }
}

impl ProgressCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.loading {
            format!("加载中... (loading={})", self.loading)
        } else {
            format!("当前进度：{:.0}%", self.current)
        }
    }

    #[command]
    pub fn on_increase(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.current = (self.current + 10.0).min(100.0);
        cx.notify();
    }

    #[command]
    pub fn on_decrease(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.current = (self.current - 10.0).max(0.0);
        cx.notify();
    }

    #[command]
    pub fn on_toggle_loading(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.loading = !self.loading;
        cx.notify();
    }
}

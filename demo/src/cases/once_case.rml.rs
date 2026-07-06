use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "framework.once",
    kind = "case",
    group = "framework",
    order = 51,
)]
#[component]
#[derive(Default)]
pub struct OnceCase {
    pub counter: u32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for OnceCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.once.title")
    }
}

impl ILifecycle for OnceCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.counter = 0;
        let (cols, rows) = build_api_table(&[
            ("once", "指令", "标记元素仅首次渲染求值，后续渲染复用首次快照"),
            ("适用场景", "说明", "静态内容、配置信息、避免重复计算"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl OnceCase {
    #[command]
    pub fn on_increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.counter = self.counter.saturating_add(1);
    }
}

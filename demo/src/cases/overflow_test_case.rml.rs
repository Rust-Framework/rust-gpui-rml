use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

/// 框架能力案例 —— 验证 CSS overflow-x / overflow-y 映射与滚动容器行为。
#[contribute(
    host_id = "demo.shell",
    id = "framework.overflow",
    kind = "case",
    group = "framework",
    order = 47,
)]
#[component]
#[derive(Default)]
pub struct OverflowTestCase {
    pub items: Vec<SharedString>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for OverflowTestCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.overflow.title")
    }
}

impl ILifecycle for OverflowTestCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.items = (1..=50)
            .map(|i| format!("条目 {i:02}：用于验证垂直滚动条与 overflow-y 映射").into())
            .collect();
        let (cols, rows) = build_api_table(&[
            ("overflow-x", "auto / hidden / scroll", "水平滚动映射"),
            ("overflow-y", "auto / hidden / scroll", "垂直滚动映射"),
            ("overflow-x-auto", "布尔标志", "等同 overflow-x: auto"),
            ("overflow-y-auto", "布尔标志", "等同 overflow-y: auto"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl OverflowTestCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- overflow_test_case.rml：验证 overflow 映射 -->
<component>
    <!-- 垂直滚动：overflow-y-auto -->
    <div class="scroll-vertical" overflow-y-auto="" style="height: 200px;">
        <p v-for="item in items">{item}</p>
    </div>

    <!-- 水平滚动：overflow-x-auto -->
    <div class="scroll-horizontal" overflow-x-auto="">
        <p>一段很长的内容用于验证水平滚动条 overflow-x 映射...</p>
    </div>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// overflow_test_case.rml.rs
use rml::prelude::*;

#[contribute(host_id = "demo.shell", id = "framework.overflow", kind = "case", group = "framework", order = 47)]
#[component]
#[derive(Default)]
pub struct OverflowTestCase {
    pub items: Vec<SharedString>,
}

impl ILifecycle for OverflowTestCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.items = (1..=50)
            .map(|i| format!("条目 {i:02}").into())
            .collect();
    }
}"#
            .to_string()
    }
}

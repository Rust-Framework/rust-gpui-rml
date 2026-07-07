use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "framework.slot_scope",
    kind = "case",
    group = "framework",
    order = 45,
)]
#[component]
#[derive(Default)]
pub struct SlotScopeCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for SlotScopeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.slot_scope.title")
    }
}

impl ILifecycle for SlotScopeCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("scope={name}", "声明", "<template slot=\"x\" scope={name}> 接收 &dyn ISlotScope"),
            ("panel.slot_name()", "查询", "返回当前插槽名（\"left\"/\"right\"/\"bottom\"）"),
            ("panel.current_size()", "查询", "返回当前尺寸（left/right 为宽，bottom 为高）"),
            ("panel.container_size()", "查询", "返回容器总尺寸（用于 maximize 计算）"),
            ("panel.has_resizable()", "查询", "是否支持 resizable 操控"),
            ("panel.maximize(window, cx)", "操作", "最大化此面板（记录原尺寸供 restore 还原）"),
            ("panel.restore(window, cx)", "操作", "还原到 maximize 之前的尺寸"),
            ("panel.close(window, cx)", "操作", "关闭/折叠此面板（尺寸调为 0 或最小阈值）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

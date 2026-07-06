use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

/// 案例组件 —— 演示 ui crate Card 组件（Ant Design 风格）。
///
/// 使用 `<Card title={...} hoverable="">` 标准卡片 API：
/// - `title` 绑定到 i18n 文本
/// - `hoverable` 启用悬浮提升
/// - body 子节点直接作为卡片内容
#[contribute(
    host_id = "demo.shell",
    id = "components.slot",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct SlotCase {
    pub card_title: String,
    pub card_body: String,
    pub hoverable: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for SlotCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.slot.title")
    }
}

impl ILifecycle for SlotCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.card_title = "动态卡片标题".into();
        self.card_body = "动态卡片内容".into();
        let (cols, rows) = build_api_table(&[
            ("title", "字符串/绑定", "卡片标题"),
            ("hoverable", "布尔标志", "悬浮提升效果"),
            ("children", "子节点", "卡片 body 内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SlotCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Card title="卡片标题" hoverable="">
    <p>卡片内容</p>
    <Button label="操作" primary="" />
</Card>"#
            .to_string()
    }

    #[command]
    pub fn on_toggle_hoverable(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.hoverable = !self.hoverable;
    }
}

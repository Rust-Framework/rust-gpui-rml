use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.button_group",
    kind = "case",
    group = "components",
    order = 28,
)]
#[component]
#[derive(Default)]
pub struct ButtonGroupCase {
    pub button_count: u8,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ButtonGroupCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.button_group.title")
    }
}

impl ILifecycle for ButtonGroupCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.button_count = 3;
        let (cols, rows) = build_api_table(&[
            ("size", "small/medium/large", "尺寸"),
            ("子节点", "Button[]", "按钮列表"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ButtonGroupCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<ButtonGroup>
    <Button label="上一步" />
    <Button label="下一步" primary="" />
</ButtonGroup>"#
            .to_string()
    }

    #[command]
    pub fn on_add_button(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        if self.button_count < 5 {
            self.button_count += 1;
        }
    }

    #[command]
    pub fn on_remove_button(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        if self.button_count > 1 {
            self.button_count -= 1;
        }
    }
}

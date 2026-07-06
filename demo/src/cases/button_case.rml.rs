use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.button",
    kind = "case",
    group = "components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub basic_clicks: i32,
    pub is_disabled: bool,
    pub is_selected: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ButtonCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.button.title")
    }
}

impl ILifecycle for ButtonCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "按钮文字"),
            ("primary / ghost / danger", "布尔标志", "变体"),
            ("disabled", "布尔", "禁用"),
            ("selected", "布尔", "选中态"),
            ("size", "small/medium/large", "尺寸"),
            ("compact", "布尔标志", "紧凑模式"),
            ("on-click", "事件", "点击回调"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ButtonCase {
    #[computed]
    pub fn basic_click_text(&self) -> String {
        format!("点击次数：{}", self.basic_clicks)
    }

    #[computed]
    pub fn disabled_status_text(&self) -> String {
        if self.is_disabled {
            "当前：禁用".to_string()
        } else {
            "当前：可用".to_string()
        }
    }

    #[computed]
    pub fn selected_status_text(&self) -> String {
        if self.is_selected {
            "当前：选中".to_string()
        } else {
            "当前：未选中".to_string()
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Button label="Default" on-click={on_basic_click} />
<Button label="Primary" primary="" on-click={on_basic_click} />
<Button label="Ghost" ghost="" on-click={on_basic_click} />
<Button label="Danger" danger="" on-click={on_basic_click} />

<Button label="Small" size="small" primary="" />
<Button label="Large" size="large" primary="" />

<Button label="Disabled" disabled={is_disabled} primary="" />
<Button label="Selected" selected={is_selected} />

<ButtonGroup>
    <Button label="上一页" />
    <Button label="下一页" />
</ButtonGroup>"#
            .to_string()
    }

    #[command]
    pub fn on_basic_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.basic_clicks += 1;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_toggle_selected(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_selected = !self.is_selected;
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.switch",
    kind = "case",
    group = "components",
    order = 34,
)]
#[component]
#[derive(Default)]
pub struct SwitchCase {
    pub is_on: bool,
    pub is_disabled: bool,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for SwitchCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.switch.title")
    }
}

impl ILifecycle for SwitchCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "标签文本"),
            ("checked", "布尔", "开关状态"),
            ("disabled", "布尔", "禁用"),
            ("size", "small/medium/large", "尺寸"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SwitchCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.is_on {
            "当前：开启".to_string()
        } else {
            "当前：关闭".to_string()
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- switch_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：checked={is_on} on-click={on_toggle} -->
    <Switch label="自动保存" checked={is_on} on-click={on_toggle} />

    <!-- 禁用状态：disabled={is_disabled} -->
    <Switch checked={is_disabled} disabled={is_disabled} />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// switch_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct SwitchCase {
    pub is_on: bool,
    pub is_disabled: bool,
}

impl SwitchCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.is_on { "开启".into() } else { "关闭".into() }
    }

    // on-click 回调签名：(&bool, &mut Context<Self>)
    #[command]
    pub fn on_toggle(&mut self, checked: &bool, cx: &mut Context<Self>) {
        self.is_on = *checked;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
        cx.notify();
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_toggle(&mut self, checked: &bool, cx: &mut Context<Self>) {
        self.is_on = *checked;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_button(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_on = !self.is_on;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
        cx.notify();
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.kbd",
    kind = "case",
    group = "components",
    order = 60,
)]
#[component]
#[derive(Default)]
pub struct KbdCase {
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for KbdCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.kbd.title")
    }
}

impl ILifecycle for KbdCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let _ = (_window, _cx);
        let (cols, rows) = build_api_table(&[
            ("key", "字符串", "按键组合（如 cmd-a / ctrl-shift-c），由 Keystroke::parse 解析"),
            ("outline", "布尔", "使用 outline 样式（默认 false）"),
            ("appearance", "布尔", "是否显示外观（默认 true，false 时仅显示文本）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl KbdCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- kbd_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 修饰键 + 字母 -->
    <Kbd key="cmd-a" />
    <Kbd key="ctrl-shift-c" />

    <!-- 功能键 -->
    <Kbd key="enter" />
    <Kbd key="escape" />

    <!-- 方向键 -->
    <Kbd key="up" />
    <Kbd key="down" />

    <!-- outline 样式 -->
    <Kbd key="cmd-a" outline="" />

    <!-- appearance=false（仅文本） -->
    <Kbd key="cmd-a" appearance="false" />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// kbd_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct KbdCase {}

impl ILifecycle for KbdCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        // Kbd 是 RenderOnce 无 ElementId 组件，无需 state
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

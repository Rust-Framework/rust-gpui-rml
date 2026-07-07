use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.collapsible",
    kind = "case",
    group = "components",
    order = 66,
)]
#[component]
#[derive(Default)]
pub struct CollapsibleCase {
    pub is_open: bool,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for CollapsibleCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.collapsible.title")
    }
}

impl ILifecycle for CollapsibleCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.is_open = true;
        let (cols, rows) = build_api_table(&[
            ("open", "bool / 绑定", "展开/折叠状态（默认 false）"),
            ("子节点", "元素", "容器内容（ParentElement）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CollapsibleCase {
    #[computed]
    pub fn state_label(&self) -> &'static str {
        if self.is_open { "已展开" } else { "已折叠" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- collapsible_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- open 静态属性 -->
    <Collapsible open="true">
        <p>默认展开的内容</p>
    </Collapsible>

    <!-- open 绑定字段 -->
    <Collapsible open={is_open}>
        <p>展开/折叠的内容</p>
    </Collapsible>

    <!-- if 配合 open 状态 -->
    <Collapsible open={is_open}>
        <h3>标题（始终显示）</h3>
        <div if={is_open}>
            <p>这部分内容使用 if 控制</p>
        </div>
    </Collapsible>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// collapsible_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct CollapsibleCase {
    pub is_open: bool,
}

impl ILifecycle for CollapsibleCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.is_open = true;
    }
}

impl CollapsibleCase {
    #[computed]
    pub fn state_label(&self) -> &'static str {
        if self.is_open { "已展开" } else { "已折叠" }
    }

    #[command]
    pub fn on_toggle(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_open = !self.is_open;
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_toggle(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_open = !self.is_open;
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

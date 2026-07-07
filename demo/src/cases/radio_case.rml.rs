use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.radio",
    kind = "case",
    group = "components",
    order = 69,
)]
#[component]
#[derive(Default)]
pub struct RadioCase {
    pub selected_index: usize,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub radio_rows: Vec<TableRow>,
}

impl IContribution for RadioCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.radio.title")
    }
}

impl ILifecycle for RadioCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.selected_index = 0;
        let (cols, group_rows) = build_api_table(&[
            ("selected_index", "usize / 绑定", "选中索引（1-based 不适用，0-based）"),
            ("horizontal / vertical / layout", "布尔标志 / horizontal|vertical", "布局方向（构造器选择）"),
            ("on-click", "事件", "选中切换回调（Fn(&usize, ...)）"),
            ("disabled", "bool / 绑定", "禁用整个组"),
            ("子节点", "Radio / 文本", "Radio 元素列表（自动转 Radio）"),
        ]);
        let (_, radio_rows) = build_api_table(&[
            ("label", "String / 绑定", "Radio 标签文本"),
            ("checked", "bool / 绑定", "选中状态（由 RadioGroup 管理）"),
            ("disabled", "bool / 绑定", "禁用"),
            ("tab_index", "isize", "Tab 顺序索引"),
            ("tab_stop", "bool", "是否参与 Tab 导航（默认 true）"),
            ("on-click", "事件", "点击回调（Fn(&bool, ...)）"),
        ]);
        self.api_columns = cols;
        self.api_rows = group_rows;
        self.radio_rows = radio_rows;
    }
}

impl RadioCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- radio_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- RadioGroup 默认垂直布局 -->
    <RadioGroup selected_index={selected_index} on-click={on_select}>
        <Radio label="选项 1" />
        <Radio label="选项 2" />
        <Radio label="选项 3" />
    </RadioGroup>

    <!-- horizontal="" 水平布局 -->
    <RadioGroup horizontal="" selected_index={selected_index} on-click={on_select}>
        <Radio label="A" />
        <Radio label="B" />
    </RadioGroup>

    <!-- layout="horizontal" 等价写法 -->
    <RadioGroup layout="horizontal" selected_index="0">
        <Radio label="X" />
        <Radio label="Y" />
    </RadioGroup>

    <!-- 文本子节点自动转 Radio -->
    <RadioGroup selected_index="0">
        文本选项 1
        文本选项 2
    </RadioGroup>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// radio_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct RadioCase {
    pub selected_index: usize,
}

impl ILifecycle for RadioCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.selected_index = 0;
    }
}

impl RadioCase {
    // RadioGroup on-click 签名为 Fn(&usize, ...)（选中索引）
    #[command]
    pub fn on_select(&mut self, idx: &usize, _cx: &mut Context<Self>) {
        self.selected_index = *idx;
    }
}"#
            .to_string()
    }

    // RadioGroup on_click 签名为 Fn(&usize, ...)（选中索引）
    #[command]
    pub fn on_select(&mut self, idx: &usize, _cx: &mut Context<Self>) {
        self.selected_index = *idx;
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

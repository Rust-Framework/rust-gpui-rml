use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.popover",
    kind = "case",
    group = "components",
    order = 62,
)]
#[component]
#[derive(Default)]
pub struct PopoverCase {
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for PopoverCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.popover.title")
    }
}

impl ILifecycle for PopoverCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let _ = (_window, _cx);
        let (cols, rows) = build_api_table(&[
            ("anchor", "枚举", "气泡定位锚点：top-left/top-center/top-right/bottom-left/bottom-center/bottom-right/left-center/right-center"),
            ("mouse-button", "枚举", "触发按键：left/right/middle，默认 left"),
            ("appearance", "bool", "是否应用默认样式（bg/border/shadow），默认 true；appearance=false 关闭"),
            ("overlay-closable", "bool", "点击外部是否关闭，默认 true；overlay-closable=false 禁用"),
            ("default-open", "bool", "初始展开状态，默认 false；default-open=true 初始展开"),
            ("slot=trigger", "slot", "标记 trigger 元素，需实现 Selectable + IntoElement（如 Button）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl PopoverCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- popover_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：slot="trigger" 标记触发元素，其余子节点为 content -->
    <Popover>
        <Button slot="trigger" label="点击展开" />
        <div v-flex="" gap-2="" p-3="">
            <p>这是气泡内容。</p>
            <p>可以放置任意元素。</p>
        </div>
    </Popover>

    <!-- 锚点定位 anchor -->
    <Popover anchor="bottom-left">
        <Button slot="trigger" label="bottom-left" />
        <div p-2="">左下角锚点</div>
    </Popover>

    <!-- 默认展开 default-open="true" -->
    <Popover default-open="true">
        <Button slot="trigger" label="已展开" />
        <div p-2="">初始展开的气泡内容</div>
    </Popover>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// popover_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct PopoverCase {}

impl ILifecycle for PopoverCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        // Popover 是无状态容器组件，无需 state
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

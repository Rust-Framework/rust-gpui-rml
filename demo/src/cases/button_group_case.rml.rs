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
    pub code_tab: usize,
    pub group_api_columns: Vec<TableColumn>,
    pub group_api_rows: Vec<TableRow>,
    pub button_api_columns: Vec<TableColumn>,
    pub button_api_rows: Vec<TableRow>,
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
        ]);
        self.group_api_columns = cols;
        self.group_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "按钮文本"),
            ("primary/ghost/danger", "布尔标志", "三种 variant"),
            ("disabled", "布尔/绑定", "禁用按钮"),
            ("selected", "布尔/绑定", "选中状态"),
            ("size", "small/medium/large", "尺寸"),
            ("compact", "布尔标志", "紧凑模式"),
            ("on-click", "事件", "点击回调（ClickEvent）"),
        ]);
        self.button_api_columns = cols;
        self.button_api_rows = rows;
    }
}

impl ButtonGroupCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- button_group_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：包裹多个 Button 子节点 -->
    <ButtonGroup>
        <Button label="上一步" />
        <Button label="下一步" primary="" />
    </ButtonGroup>

    <!-- 操作分组：variant 混合 -->
    <ButtonGroup>
        <Button label="保存" primary="" />
        <Button label="取消" ghost="" />
        <Button label="删除" danger="" />
    </ButtonGroup>

    <!-- 动态 if 条件渲染：根据 button_count 增减按钮 -->
    <ButtonGroup>
        <Button label="按钮 1" if={button_count >= 1} />
        <Button label="按钮 2" primary="" if={button_count >= 2} />
        <Button label="按钮 3" ghost="" if={button_count >= 3} />
    </ButtonGroup>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// button_group_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct ButtonGroupCase {
    pub button_count: u8,
}

impl ILifecycle for ButtonGroupCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.button_count = 3;
    }
}

impl ButtonGroupCase {
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
}"#
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

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

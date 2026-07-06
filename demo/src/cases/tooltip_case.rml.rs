use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.tooltip",
    kind = "case",
    group = "components",
    order = 61,
)]
#[component]
#[derive(Default)]
pub struct TooltipCase {
    pub tooltip_text: SharedString,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl TooltipCase {
    #[computed]
    pub fn dynamic_tooltip(&self) -> SharedString {
        self.tooltip_text.clone()
    }
}

impl IContribution for TooltipCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tooltip.title")
    }
}

impl ILifecycle for TooltipCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let _ = (_window, _cx);
        self.tooltip_text = "动态 Tooltip 内容".into();
        let (cols, rows) = build_api_table(&[
            ("tooltip", "字符串", "悬浮提示文本，生成 .tooltip(\"text\")，仅支持特定组件"),
            ("支持组件", "枚举", "Button / IconButton / DropdownButton / Toggle / Checkbox / Clipboard / Radio / Switch"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TooltipCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- tooltip_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 静态 tooltip="text" -->
    <Button label="保存" tooltip="保存文件 (Cmd+S)" />
    <Button label="删除" tooltip="删除选中项" />

    <!-- 动态绑定 tooltip={dynamic_tooltip} -->
    <Button label="撤销" tooltip={dynamic_tooltip} />

    <!-- Checkbox / Switch 等组件的 tooltip -->
    <Checkbox tooltip="接受服务条款">同意条款</Checkbox>
    <Switch tooltip="切换深色模式" />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// tooltip_case.rml.rs：后端状态 + computed + command handler
use gpui::SharedString;
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct TooltipCase {
    pub tooltip_text: SharedString,
}

impl ILifecycle for TooltipCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.tooltip_text = "动态 Tooltip 内容".into();
    }
}

impl TooltipCase {
    // computed 方法返回 SharedString，供 tooltip={dynamic_tooltip} 绑定
    #[computed]
    pub fn dynamic_tooltip(&self) -> SharedString {
        self.tooltip_text.clone()
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

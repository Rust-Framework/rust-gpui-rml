use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{InputState, TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.input",
    kind = "case",
    group = "components",
    order = 35,
)]
#[component]
#[derive(Default)]
pub struct InputCase {
    /// 通过 `ref="input_state"` 指令关联，
    /// 首次渲染后由 `__rml_populate_refs` 注入 `Entity<InputState>` 句柄。
    ///
    /// placeholder 通过 InputState builder 在 on_loaded 中设置
    ///（Input element 不直接接收 placeholder 属性，需在 state 上设置）。
    pub input_state: ElementRef<InputState>,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for InputCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.input.title")
    }
}

impl ILifecycle for InputCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        // Entity<InputState> 在首次渲染时由 `ref` 指令惰性创建。
        // on_loaded 阶段 ref_entities 尚未填充，故 ElementRef 仍为空，
        // placeholder 等需要在 InputState 上设置的属性，
        // 应在首次 render 后通过 ElementRef.with_mut 设置，
        // 或后续通过其他生命周期钩子（如 on_rendered，待 M5' 实现）。
        let _ = (_window, cx);
        let (cols, rows) = build_api_table(&[
            ("placeholder", "字符串", "占位文本（InputState builder）"),
            ("default_value", "字符串", "默认值（InputState builder）"),
            ("disabled", "布尔", "禁用（Input 组件属性）"),
            ("ref", "字符串", "元素引用名（绑定到 ElementRef<T> 字段）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl InputCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- input_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：ref="input_state" 惰性创建 Entity<InputState> -->
    <Input ref="input_state" />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// input_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;
use rml_ui::InputState;

#[component]
#[derive(Default)]
pub struct InputCase {
    // ref="input_state" 注入 Entity<InputState> 句柄
    pub input_state: ElementRef<InputState>,
}

impl ILifecycle for InputCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        // InputState Entity 在首次渲染时由 ref 指令惰性创建
        // placeholder 等需在 state 上设置，应在首次 render 后通过
        // ElementRef.with_mut 设置
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

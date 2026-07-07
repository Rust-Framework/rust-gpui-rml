use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.group_box",
    kind = "case",
    group = "components",
    order = 67,
)]
#[component]
#[derive(Default)]
pub struct GroupBoxCase {
    pub dynamic_title: String,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for GroupBoxCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.group_box.title")
    }
}

impl ILifecycle for GroupBoxCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.dynamic_title = "动态标题 1".into();
        let (cols, rows) = build_api_table(&[
            ("title", "String / 绑定", "标题（impl IntoElement）"),
            ("normal / fill / outline", "布尔标志", "3 种 variant（构造器选择）"),
            ("variant", "normal/fill/outline", "variant 属性（builder 方法 .with_variant(...)）"),
            ("子节点", "元素", "分组内容（ParentElement）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl GroupBoxCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- group_box_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- normal 默认（无 variant 属性） -->
    <GroupBox title="用户信息">
        <p>用户名：admin</p>
    </GroupBox>

    <!-- fill 填充背景 -->
    <GroupBox title="设置面板" fill="">
        <p>主题：暗色</p>
    </GroupBox>

    <!-- outline 描边 -->
    <GroupBox title="高级选项" outline="">
        <p>启用通知：是</p>
    </GroupBox>

    <!-- variant 字符串属性 -->
    <GroupBox title="示例" variant="fill">
        <p>通过 variant 属性设置</p>
    </GroupBox>

    <!-- title 绑定字段 -->
    <GroupBox title={dynamic_title} fill="">
        <p>动态标题内容</p>
    </GroupBox>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// group_box_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct GroupBoxCase {
    pub dynamic_title: String,
}

impl ILifecycle for GroupBoxCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.dynamic_title = "动态标题 1".into();
    }
}

impl GroupBoxCase {
    #[command]
    pub fn on_cycle_title(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.dynamic_title = if self.dynamic_title.contains("1") {
            "动态标题 2".into()
        } else {
            "动态标题 1".into()
        };
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_cycle_title(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.dynamic_title = if self.dynamic_title.contains("1") {
            "动态标题 2".into()
        } else {
            "动态标题 1".into()
        };
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

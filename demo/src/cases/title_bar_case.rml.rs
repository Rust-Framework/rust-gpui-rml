use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.title_bar",
    kind = "case",
    group = "components",
    order = 31,
)]
#[component]
#[derive(Default)]
pub struct TitleBarCase {
    pub title: String,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for TitleBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.title_bar.title")
    }
}

impl ILifecycle for TitleBarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.title = "RML Showcase".into();
        let (cols, rows) = build_api_table(&[
            ("子节点", "元素[]", "中央区域内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TitleBarCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- title_bar_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：TitleBar 自动渲染窗口控制按钮 -->
    <TitleBar>
        <span>RML Showcase</span>
    </TitleBar>

    <!-- 动态绑定：model 双向绑定 + computed -->
    <input model={title} placeholder="输入标题文本" />
    <p>当前标题: {title}</p>
    <Button label="重置标题" on-click={on_reset_title} />
    <TitleBar>
        <span>{title}</span>
    </TitleBar>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// title_bar_case.rml.rs：后端状态 + computed + command handler
use gpui::SharedString;
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct TitleBarCase {
    pub title: String,
}

impl ILifecycle for TitleBarCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.title = "RML Showcase".into();
    }
}

impl TitleBarCase {
    // on-reset-title 回调签名：(&ClickEvent, &mut Context<Self>)
    #[command]
    pub fn on_reset_title(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.title = "RML Showcase".into();
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_reset_title(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.title = "RML Showcase".into();
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.native_status_bar",
    kind = "case",
    group = "components",
    order = 32,
)]
#[component]
#[derive(Default)]
pub struct NativeStatusBarCase {
    pub status_text: String,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for NativeStatusBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.native_status_bar.title")
    }
}

impl ILifecycle for NativeStatusBarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.status_text = "就绪".into();
        let (cols, rows) = build_api_table(&[
            ("子节点", "元素[]", "中央区域内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl NativeStatusBarCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- native_status_bar_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：NativeStatusBar 包裹子元素 -->
    <NativeStatusBar>
        <span>就绪</span>
    </NativeStatusBar>

    <!-- 动态状态：on-click 切换 status_text -->
    <Button label="就绪" primary="" on-click={on_show_ready} />
    <Button label="警告" warning="" on-click={on_show_warning} />
    <Button label="错误" danger="" on-click={on_show_error} />
    <p>当前状态: {status_text}</p>
    <NativeStatusBar>
        <span>{status_text}</span>
    </NativeStatusBar>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// native_status_bar_case.rml.rs：后端状态 + computed + command handler
use gpui::SharedString;
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct NativeStatusBarCase {
    pub status_text: String,
}

impl ILifecycle for NativeStatusBarCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.status_text = "就绪".into();
    }
}

impl NativeStatusBarCase {
    // 三个 #[command] handler 分别切换状态文本
    #[command]
    pub fn on_show_ready(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "就绪".into();
    }

    #[command]
    pub fn on_show_warning(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "警告:请检查配置".into();
    }

    #[command]
    pub fn on_show_error(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "错误:连接失败".into();
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_show_ready(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "就绪".into();
    }

    #[command]
    pub fn on_show_warning(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "警告:请检查配置".into();
    }

    #[command]
    pub fn on_show_error(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "错误:连接失败".into();
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.tab_bar",
    kind = "case",
    group = "components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct TabBarCase {
    pub active_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for TabBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tab_bar.title")
    }
}

impl ILifecycle for TabBarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("selected-index", "绑定", "当前选中索引"),
            ("on-click", "事件", "点击回调，签名 fn(index: usize)"),
            ("underline/pill/flat/outline/segmented", "布尔标志", "5 种 variant"),
            ("menu", "布尔", "启用下拉菜单（标签过多时）"),
            ("prefix/suffix", "绑定", "首尾注入元素"),
            ("Tab label", "字符串", "标签标题"),
            ("Tab icon", "图标名", "标签图标"),
            ("Tab disabled", "布尔", "禁用标签"),
            ("Tab selected", "布尔", "选中状态"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TabBarCase {
    #[computed]
    pub fn status_text(&self) -> String {
        format!("当前选中索引：{}", self.active_tab)
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<TabBar selected_index={active_tab} on_click={on_tab_select}>
    <Tab label="Account" />
    <Tab label="Profile" />
</TabBar>

<!-- TabItem (WPF TabControl 模式)：title + body -->
<TabBar selected_index={active_tab} on_click={on_tab_select}>
    <tab-item title="Account">
        <div>Account settings panel</div>
    </tab-item>
    <tab-item title="Profile">
        <div>User profile panel</div>
    </tab-item>
</TabBar>"#
            .to_string()
    }

    #[command]
    pub fn on_tab_select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }
}

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
    pub code_tab: usize,
    pub tab_bar_api_columns: Vec<TableColumn>,
    pub tab_bar_api_rows: Vec<TableRow>,
    pub tab_api_columns: Vec<TableColumn>,
    pub tab_api_rows: Vec<TableRow>,
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
            ("on-close", "事件", "关闭按钮回调，签名 fn(index: usize)"),
            ("on-close-all", "事件", "关闭全部回调，签名 fn()"),
            ("on-close-others", "事件", "关闭其他回调，签名 fn(index: usize)"),
            ("on-promote", "事件", "双击 promote 回调，签名 fn(index: usize)"),
            ("underline/pill/flat/outline/segmented", "布尔标志", "5 种 variant"),
            ("menu", "布尔", "启用下拉菜单（标签过多时）"),
            ("prefix/suffix", "绑定", "首尾注入元素"),
            ("last-empty-space", "绑定", "尾部占位元素"),
            ("track-scroll", "绑定", "滚动控制（ScrollHandle 引用）"),
        ]);
        self.tab_bar_api_columns = cols;
        self.tab_bar_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("label", "字符串/绑定", "标签标题（底层映射 TabItem::title）"),
            ("icon", "图标名", "标签图标（底层映射 TabItem::title_icon）"),
            ("disabled", "布尔/绑定", "禁用标签"),
            ("closable", "布尔/绑定", "显示关闭按钮"),
            ("preview", "布尔/绑定", "预览模式（italic 标题）"),
            ("on-click", "事件", "点击回调（ClickEvent）"),
            ("子节点", "内容", "element 子节点作为 body（选中时渲染，WPF TabItem 模式）"),
            ("template slot=\"header\"", "插槽", "header 自定义插槽（覆盖 label/icon）"),
        ]);
        self.tab_api_columns = cols;
        self.tab_api_rows = rows;
    }
}

impl TabBarCase {
    #[computed]
    pub fn status_text(&self) -> String {
        format!("当前选中索引：{}", self.active_tab)
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- tab_bar_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：selected-index={active_tab} on-click={on_tab_select} -->
    <TabBar selected-index={active_tab} on-click={on_tab_select}>
        <Tab label="Account" />
        <Tab label="Profile" />
        <Tab label="Settings" />
    </TabBar>

    <!-- 5 种 variant：underline/pill/flat/outline/segmented -->
    <TabBar underline="">
        <Tab label="Underline" />
    </TabBar>
    <TabBar pill="">
        <Tab label="Pill" />
    </TabBar>
    <TabBar segmented="">
        <Tab label="Segmented" />
    </TabBar>

    <!-- 尺寸 size -->
    <TabBar size="small">
        <Tab label="small" />
    </TabBar>

    <!-- 带图标 -->
    <TabBar>
        <Tab icon="User" label="Account" />
        <Tab icon="Bell" label="Notifications" />
    </TabBar>

    <!-- 禁用/选中（选中状态由 TabBar::selected-index 控制） -->
    <TabBar>
        <Tab label="Normal" />
        <Tab label="Disabled" disabled="true" />
    </TabBar>

    <!-- menu 模式（标签过多时启用下拉） -->
    <TabBar menu="true">
        <Tab label="Tab 1" />
        <Tab label="Tab 2" />
        <Tab label="Tab 3" />
    </TabBar>

    <!-- header 自定义插槽：template slot="header" 注入任意标题元素 -->
    <TabBar selected-index={active_tab} on-click={on_tab_select}>
        <Tab>
            <template slot="header">
                <span>Account</span>
                <Badge>3</Badge>
            </template>
        </Tab>
        <Tab>
            <template slot="header">
                <span>Profile</span>
            </template>
        </Tab>
    </TabBar>

    <!-- 内容面板 body：Tab 直接包裹 element 子节点（WPF TabControl/TabItem 模式） -->
    <TabBar selected-index={active_tab} on-click={on_tab_select}>
        <Tab label="Account">
            <div>Account settings panel</div>
        </Tab>
        <Tab label="Profile">
            <div>User profile panel</div>
        </Tab>
    </TabBar>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// tab_bar_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct TabBarCase {
    pub active_tab: usize,
}

impl TabBarCase {
    #[computed]
    pub fn status_text(&self) -> String {
        format!("当前选中索引：{}", self.active_tab)
    }

    // on-click 回调签名：fn(index: usize, &mut Context<Self>)
    #[command]
    pub fn on_tab_select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_tab_select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

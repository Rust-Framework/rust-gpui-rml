use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.tab_bar",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct TabBarCase {
    pub active_tab: usize,
    pub tabs_api_columns: Vec<TableColumn>,
    pub tabs_api_rows: Vec<TableRow>,
    pub tab_bar_api_columns: Vec<TableColumn>,
    pub tab_bar_api_rows: Vec<TableRow>,
    pub tab_api_columns: Vec<TableColumn>,
    pub tab_api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("selected-index", "绑定", "当前选中索引"),
            ("on-click", "事件", "点击回调，签名 fn(index: usize)"),
            ("on-close", "事件", "关闭按钮回调，签名 fn(index: usize)"),
            ("on-close-all", "事件", "关闭全部回调，签名 fn()"),
            ("on-close-others", "事件", "关闭其他回调，签名 fn(index: usize)"),
            ("on-promote", "事件", "双击 promote 回调，签名 fn(index: usize)"),
            ("bordered", "布尔标志", "1px 边框包裹 header + body 整体（Tabs 专属）"),
            ("underline/pill/flat/outline/segmented", "布尔标志", "5 种 variant"),
            ("menu", "布尔", "启用下拉菜单 + 溢出压缩（标签过多时）"),
            ("prefix/suffix", "绑定", "首尾注入元素"),
            ("last-empty-space", "绑定", "尾部占位元素"),
            ("track-scroll", "绑定", "滚动控制（ScrollHandle 引用）"),
        ]);
        self.tabs_api_columns = cols;
        self.tabs_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("selected-index", "绑定", "当前选中索引"),
            ("on-click", "事件", "点击回调，签名 fn(index: usize)"),
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

    /// TabBar + 手动 body 面板：按 active_tab 显示对应内容
    #[computed]
    pub fn basic_body_text(&self) -> &'static str {
        match self.active_tab {
            0 => "Account settings — 管理账号、密码与安全选项。",
            1 => "User profile — 头像、简介与公开资料。",
            _ => "System settings — 通知、语言与默认偏好。",
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("tab_bar_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("tab_bar_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_tab_select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }
}

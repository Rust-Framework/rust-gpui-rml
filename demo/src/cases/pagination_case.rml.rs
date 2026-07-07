use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.pagination",
    kind = "case",
    group = "components",
    order = 68,
)]
#[component]
#[derive(Default)]
pub struct PaginationCase {
    pub current_page: usize,
    pub total_pages: usize,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for PaginationCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.pagination.title")
    }
}

impl ILifecycle for PaginationCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.current_page = 1;
        self.total_pages = 10;
        let (cols, rows) = build_api_table(&[
            ("current_page", "usize / 绑定", "当前页码（1-based）"),
            ("total_pages", "usize / 绑定", "总页数"),
            ("visible_pages", "usize", "最大可见页码数（默认 5）"),
            ("compact", "布尔标志", "紧凑模式（仅前后箭头）"),
            ("on-click", "事件", "页码切换回调（Fn(&usize, ...)）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait）"),
            ("disabled", "bool / 绑定", "禁用（Disableable trait）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl PaginationCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- pagination_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：current_page/total_pages 绑定字段 -->
    <Pagination
        current_page={current_page}
        total_pages={total_pages}
        on-click={on_page_change}
    />

    <!-- visible_pages 控制显示页码数 -->
    <Pagination current_page="3" total_pages="20" visible_pages="7" />

    <!-- compact 紧凑模式 -->
    <Pagination current_page="1" total_pages="10" compact="" />

    <!-- 静态属性（无绑定，不会自动更新） -->
    <Pagination current_page="3" total_pages="10" />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// pagination_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct PaginationCase {
    pub current_page: usize,
    pub total_pages: usize,
}

impl ILifecycle for PaginationCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.current_page = 1;
        self.total_pages = 10;
    }
}

impl PaginationCase {
    // on-click 事件签名为 Fn(&usize, ...)（页码），不是 ClickEvent
    #[command]
    pub fn on_page_change(&mut self, page: &usize, _cx: &mut Context<Self>) {
        self.current_page = *page;
    }
}"#
            .to_string()
    }

    // on_click 事件签名为 Fn(&usize, ...)（页码）
    #[command]
    pub fn on_page_change(&mut self, page: &usize, _cx: &mut Context<Self>) {
        self.current_page = *page;
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}

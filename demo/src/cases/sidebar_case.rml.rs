use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.sidebar",
    kind = "case",
    group = "components",
    order = 87,
)]
#[component]
#[derive(Default)]
pub struct SidebarCase {
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub is_collapsed: bool,
    pub sidebar_api_columns: Vec<TableColumn>,
    pub sidebar_api_rows: Vec<TableRow>,
    pub menu_api_columns: Vec<TableColumn>,
    pub menu_api_rows: Vec<TableRow>,
    pub item_api_columns: Vec<TableColumn>,
    pub item_api_rows: Vec<TableRow>,
}

impl IContribution for SidebarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.sidebar.title")
    }
}

impl ILifecycle for SidebarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.is_collapsed = false;

        let (cols, rows) = build_api_table(&[
            ("side", "left/right", "侧边栏位置（默认 left）"),
            ("collapsible", "icon/offcanvas/none", "折叠模式（icon 图标折叠，offcanvas 抽屉，none 不可折叠）"),
            ("collapsed", "bool 绑定", "折叠状态（受控模式，配合 collapsible 使用）"),
            ("header", "slot 插槽", "顶部内容插槽（slot=\"header\"）"),
            ("footer", "slot 插槽", "底部内容插槽（slot=\"footer\"）"),
            ("ref", "指令", "稳定 ElementId（ref=\"name\" → \"rml_ref:name\"）"),
            ("SidebarMenu", "子节点", "菜单分组容器，通过 SidebarEntry::Menu 包装"),
            ("SidebarMenuItem", "子节点", "菜单项，通过 SidebarEntry::Item 包装"),
        ]);
        self.sidebar_api_columns = cols;
        self.sidebar_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("（无专用属性）", "—", "SidebarMenu 仅实现 Styled，支持通用 CSS 样式属性"),
            ("SidebarMenuItem", "子节点", "菜单项子节点，通过 .child() 注入"),
        ]);
        self.menu_api_columns = cols;
        self.menu_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("label", "字符串/绑定", "菜单项标签（构造器参数，非 setter）"),
            ("icon", "IconName 枚举值", "菜单项图标（如 Home/Settings）"),
            ("active", "布尔", "高亮选中状态（active=\"\" 为 true）"),
            ("default-open", "布尔", "子菜单默认展开（default-open=\"\" 为 true）"),
            ("click-to-open", "布尔", "点击打开子菜单（click-to-open=\"\" 为 true）"),
            ("click-to-toggle", "布尔", "点击切换子菜单展开/折叠"),
            ("disabled", "布尔", "禁用菜单项（映射到 .disable() 方法）"),
            ("on-click", "事件", "点击事件回调"),
            ("SidebarMenuItem", "子节点", "子菜单项（通过 .children(vec![...]) 注入）"),
        ]);
        self.item_api_columns = cols;
        self.item_api_rows = rows;
    }
}

impl SidebarCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("sidebar_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("sidebar_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_item_click(&mut self, _event: &ClickEvent, cx: &mut Context<Self>) {
        cx.notify();
    }

    #[command]
    pub fn on_toggle_collapse(&mut self, _event: &ClickEvent, cx: &mut Context<Self>) {
        self.is_collapsed = !self.is_collapsed;
        cx.notify();
    }
}

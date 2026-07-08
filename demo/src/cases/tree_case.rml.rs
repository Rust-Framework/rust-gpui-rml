use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow, TreeItem, TreeState};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.tree",
    kind = "case",
    group = "components",
    order = 36,
)]
#[component]
#[derive(Default)]
pub struct TreeCase {
    /// Tree 不支持 ref 指令（gen_tree 硬编码 self.tree_state.as_ref()），
    /// 必须在 on_loaded 中手动 cx.new 创建 TreeState Entity 并设置 items。
    /// 字段名必须为 tree_state（tags.rs 中 Tree 的 state_field 硬编码）。
    pub tree_state: Option<gpui::Entity<TreeState>>,

    /// on_activate 仅叶子节点触发，记录最后激活的叶子 id
    pub last_activated: SharedString,

    /// on_select 含文件夹节点触发，记录最后选中的节点 id
    pub last_selected: SharedString,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TreeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tree.title")
    }
}

impl ILifecycle for TreeCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        // 在 on_loaded 中创建 TreeState Entity 并配置 items。
        // TreeItem 通过 builder 模式构建：
        // - new(id, label) 创建节点
        // - .child(...) 添加单个子节点（可链式）
        // - .expanded(true) 设置初始展开
        // - .disabled(true) 禁用节点（不触发事件）
        // is_folder() 根据 children 是否为空自动判断。
        let items = vec![
            TreeItem::new("group1", "分组一")
                .expanded(true)
                .child(TreeItem::new("item1", "选项 1"))
                .child(TreeItem::new("item2", "选项 2")),
            TreeItem::new("group2", "分组二")
                .child(TreeItem::new("item3", "选项 3"))
                .child(TreeItem::new("item4", "选项 4")),
            TreeItem::new("item5", "独立项"),
            TreeItem::new("disabled_item", "禁用项（disabled）").disabled(true),
            // 多层级嵌套：root → child → grandchild
            TreeItem::new("nested_root", "嵌套根")
                .child(
                    TreeItem::new("nested_child", "嵌套子")
                        .child(TreeItem::new("nested_grandchild", "嵌套孙")),
                ),
        ];
        self.tree_state = Some(cx.new(|cx| TreeState::new(cx).items(items)));

        let (cols, rows) = build_api_table(&[
            ("on-activate", "事件", "叶子节点激活回调（参数：&SharedString item_id；文件夹节点不触发）"),
            ("on-select", "事件", "节点选中回调（参数：&SharedString item_id；含文件夹节点）"),
            ("tree_state", "Option<Entity<TreeState>>", "状态字段（tags.rs state_field 硬编码，on_loaded 中创建）"),
            ("TreeState::new", "构造器", "TreeState::new(cx) 创建空 state（cx: &mut App）"),
            ("TreeState::items", "builder", ".items(Vec<TreeItem>) 设置树节点（builder 模式，仅创建时）"),
            ("TreeItem::new", "构造器", "TreeItem::new(id: impl Into<SharedString>, label: impl Into<SharedString>)"),
            ("TreeItem::child", "builder", ".child(TreeItem) 添加单个子节点（可链式）"),
            ("TreeItem::children", "builder", ".children(impl IntoIterator<Item = TreeItem>) 添加多个子节点"),
            ("TreeItem::expanded", "builder", ".expanded(bool) 设置初始展开状态"),
            ("TreeItem::disabled", "builder", ".disabled(bool) 禁用节点（不触发事件）"),
            ("TreeItem::is_folder", "查询", ".is_folder() -> bool（根据 children 自动判断）"),
            ("TreeItem::is_expanded", "查询", ".is_expanded() -> bool"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TreeCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("tree_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("tree_case.rml.rs").to_string()
    }

    /// on_activate 仅叶子节点触发，记录最后激活的叶子 id
    #[computed]
    pub fn activated_text(&self) -> String {
        if self.last_activated.is_empty() {
            "未激活".to_string()
        } else {
            self.last_activated.to_string()
        }
    }

    /// on_select 含文件夹节点触发，记录最后选中的节点 id
    #[computed]
    pub fn selected_text(&self) -> String {
        if self.last_selected.is_empty() {
            "未选中".to_string()
        } else {
            self.last_selected.to_string()
        }
    }

    /// on_activate 回调签名：(&SharedString, &mut Context<Self>)
    /// 仅叶子节点点击触发，文件夹节点点击仅展开/折叠不触发
    #[command]
    pub fn on_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        self.last_activated = item_id.clone();
        cx.notify();
    }

    /// on_select 回调签名：(&SharedString, &mut Context<Self>)
    /// 含文件夹节点触发，点击文件夹也会触发
    #[command]
    pub fn on_select(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        self.last_selected = item_id.clone();
        cx.notify();
    }
}

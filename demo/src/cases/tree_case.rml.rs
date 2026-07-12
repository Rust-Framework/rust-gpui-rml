use gpui::SharedString;
use rml::prelude::*;
use rml_core::element_ref::ElementRef;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow, TreeData, TreeState};

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
    /// Tree 通过 ref="basic_tree" 引用，items 绑定 tree_items 字段
    pub basic_tree: ElementRef<TreeState>,
    pub tree_items: Vec<TreeData>,

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

        // TreeData 通过 builder 模式构建：
        // - new(id, label) 创建节点
        // - .child(...) 添加单个子节点（可链式）
        // - .expanded(true) 设置初始展开
        // - .disabled(true) 禁用节点（不触发事件）
        // is_folder() 根据 children 是否为空自动判断。
        self.tree_items = vec![
            TreeData::new("group1", "分组一")
                .expanded(true)
                .child(TreeData::new("item1", "选项 1"))
                .child(TreeData::new("item2", "选项 2")),
            TreeData::new("group2", "分组二")
                .child(TreeData::new("item3", "选项 3"))
                .child(TreeData::new("item4", "选项 4")),
            TreeData::new("item5", "独立项"),
            TreeData::new("disabled_item", "禁用项（disabled）").disabled(true),
            // 多层级嵌套：root → child → grandchild
            TreeData::new("nested_root", "嵌套根")
                .child(
                    TreeData::new("nested_child", "嵌套子")
                        .child(TreeData::new("nested_grandchild", "嵌套孙")),
                ),
        ];

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段，如 ref=\"basic_tree\""),
            ("items", "binding", "树节点数据，如 items={tree_items}"),
            ("on-activate", "event", "叶子节点激活时回调，参数为节点 id；文件夹节点不触发"),
            ("on-select", "event", "节点选中时回调，参数为节点 id；含文件夹节点"),
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

use gpui::SharedString;
use rml::prelude::*;
use rml_core::element_ref::ElementRef;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow, TreeData, TreeState};

use crate::cases::common::{build_api_table, CaseDocPage};

/// 虚拟树 demo：10000+ 节点流畅渲染
///
/// gpui-component Tree 内部已使用 `uniform_list` 实现虚拟化，仅渲染可见范围。
/// 本 demo 通过生成 10000+ 节点的大型树验证性能。
#[contribute(
    host_id = "demo.shell",
    id = "components.virtual_tree",
    kind = "case",
    group = "components",
    order = 74,
)]
#[component]
#[derive(Default)]
pub struct VirtualTreeCase {
    /// Tree 通过 ref 引用 TreeState，items 绑定 tree_items 字段
    pub big_tree: ElementRef<TreeState>,
    pub tree_items: Vec<TreeData>,

    /// 节点总数（显示用）
    pub total_nodes: usize,

    /// 最后激活的叶子 id
    pub last_activated: SharedString,

    /// 最后选中的节点 id
    pub last_selected: SharedString,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for VirtualTreeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.virtual_tree.title")
    }
}

impl ILifecycle for VirtualTreeCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        // 生成 10000+ 节点的大型树：
        // 200 个根文件夹，每个根含 50 个叶子节点 → 200 + 200*50 = 10200 节点
        // 前 3 个根初始展开，展开后可见 ~153 节点（3*50 + 200）
        const ROOT_COUNT: usize = 200;
        const LEAVES_PER_ROOT: usize = 50;

        let mut total = 0usize;
        let mut roots: Vec<TreeData> = Vec::with_capacity(ROOT_COUNT);
        for r in 0..ROOT_COUNT {
            let mut root = TreeData::new(format!("root-{r}"), format!("目录 {r}"));
            if r < 3 {
                root = root.expanded(true);
            }
            for l in 0..LEAVES_PER_ROOT {
                let leaf = TreeData::new(
                    format!("leaf-{r}-{l}"),
                    format!("文件 {r}-{l}.rs"),
                );
                root = root.child(leaf);
                total += 1;
            }
            total += 1;
            roots.push(root);
        }
        self.tree_items = roots;
        self.total_nodes = total;

        let (cols, rows) = build_api_table(&[
            ("ref", "字符串", "TreeState Entity 引用名（配合 items 声明式绑定）"),
            ("items", "Vec<TreeData> 绑定", "大型树节点数据（委托注入 TreeState，内部 uniform_list 自动虚拟化）"),
            ("on-activate", "事件", "叶子节点激活回调（参数：&SharedString item_id）"),
            ("on-select", "事件", "节点选中回调（参数：&SharedString item_id；含文件夹）"),
            ("虚拟化", "内置", "gpui-component Tree 通过 uniform_list 仅渲染可见范围，无需额外开关"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl VirtualTreeCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("virtual_tree_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("virtual_tree_case.rml.rs").to_string()
    }

    #[computed]
    pub fn activated_text(&self) -> String {
        if self.last_activated.is_empty() {
            "未激活".to_string()
        } else {
            self.last_activated.to_string()
        }
    }

    #[computed]
    pub fn selected_text(&self) -> String {
        if self.last_selected.is_empty() {
            "未选中".to_string()
        } else {
            self.last_selected.to_string()
        }
    }

    #[command]
    pub fn on_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        self.last_activated = item_id.clone();
        cx.notify();
    }

    #[command]
    pub fn on_select(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        self.last_selected = item_id.clone();
        cx.notify();
    }
}

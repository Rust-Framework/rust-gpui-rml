//! 案例目录 —— Tree 数据与元信息

use gpui::AppContext;
use rml_app::contribution::{build_contribution_tree, ContributionRegistryGlobal, ContributionTreeNode};

use crate::shell::hosts;
use rml_ui::{TreeItem, TreeState};

/// 已打开的 Tab 页签
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenTab {
    pub id: String,
    pub title: String,
}

/// 案例标题 i18n key
pub fn case_title_key(id: &str) -> &'static str {
    match id {
        "welcome" => "shell.welcome",
        "binding.counter" => "case.counter.title",
        "binding.two-way" => "case.two_way.title",
        "components.button" => "case.button.title",
        "i18n.basic" => "case.i18n.title",
        _ => "shell.case_default",
    }
}

fn contribution_node_to_tree_item(node: &ContributionTreeNode) -> TreeItem {
    let mut item = TreeItem::new(node.id.clone(), node.name.clone());
    if !node.children.is_empty() {
        item = item.expanded(true);
        for child in &node.children {
            item = item.child(contribution_node_to_tree_item(child));
        }
    }
    item
}

/// 从案例树 host 贡献点构建树（`parent_id` 层级，纯数据消费）
pub fn tree_items_from_contributions<C>(cx: &gpui::Context<C>) -> Vec<TreeItem> {
    let registry = &cx.global::<ContributionRegistryGlobal>().0;
    build_contribution_tree(registry, hosts::CASE_TREE)
        .iter()
        .map(contribution_node_to_tree_item)
        .collect()
}

/// 在 `on_loaded` 中初始化案例树状态
pub fn init_tree_state<C>(cx: &mut gpui::Context<C>) -> gpui::Entity<TreeState> {
    cx.new(|cx| TreeState::new(cx).items(tree_items_from_contributions(cx)))
}

/// 切换语言后刷新案例树
pub fn refresh_tree_state<C>(state: &gpui::Entity<TreeState>, cx: &mut gpui::Context<C>) {
    state.update(cx, |tree, cx| {
        tree.set_items(tree_items_from_contributions(cx), cx);
    });
}

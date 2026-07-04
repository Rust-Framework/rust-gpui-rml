//! 贡献条目投影：Shell 控件数据构建。
//!
//! 仅保留菜单树构建（`build_menu_tree`）+ `ContribEntry` 类型别名。
//! status/activity/case 投影已迁入 MainWindow 内联或 CaseViewModel。

use std::collections::HashMap;
use std::sync::Arc;

use rml_core::command::CommandAbilityExt;
use rml_core::contribution::{ContributionOptions, IContribution};
use rml_ui::{IMenuItem, MenuItem};

pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions);

/// 菜单树节点（合并 submenu root 与 leaf command）
struct MenuNode {
    id: String,
    name: gpui::SharedString,
    order: i32,
    parent_id: Option<String>,
    /// 叶子节点携带贡献引用（submenu root 为 None）
    contribution: Option<Arc<dyn IContribution>>,
}

/// 从 menu 槽位贡献构建菜单树（按 parent_id 建树，按 order 排序）。
pub fn build_menu_tree(entries: &[ContribEntry]) -> Vec<Arc<dyn IMenuItem>> {
    let mut all: Vec<MenuNode> = Vec::new();

    for (c, o) in entries
        .iter()
        .filter(|(_, o)| o.effective_slot() == Some("menu"))
    {
        all.push(MenuNode {
            id: c.id().to_string(),
            name: c.name(),
            order: o.order,
            parent_id: o.parent_id.as_ref().map(|s| s.to_string()),
            contribution: c.as_command().map(|_| c.clone()),
        });
    }

    let mut by_parent: HashMap<Option<String>, Vec<&MenuNode>> = HashMap::new();
    for node in &all {
        by_parent
            .entry(node.parent_id.clone())
            .or_default()
            .push(node);
    }

    fn build_children(
        parent_id: Option<&str>,
        by_parent: &HashMap<Option<String>, Vec<&MenuNode>>,
    ) -> Vec<Arc<dyn IMenuItem>> {
        let key = parent_id.map(|s| s.to_string());
        let mut siblings = by_parent.get(&key).cloned().unwrap_or_default();
        siblings.sort_by_key(|n| n.order);
        siblings
            .into_iter()
            .map(|node| {
                let mut item = MenuItem::new(node.name.clone());
                if let Some(c) = &node.contribution {
                    item = item.contribution(c.clone());
                }
                let children = build_children(Some(&node.id), by_parent);
                if !children.is_empty() {
                    item = item.children(children);
                }
                item.into_arc()
            })
            .collect()
    }

    build_children(None, &by_parent)
}

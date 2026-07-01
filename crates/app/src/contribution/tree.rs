//! 从层级贡献（`parent_id`）构建树形 UI 数据

use std::collections::HashMap;

use gpui::SharedString;
use rml_core::contribution::{ContributedEntry, IContributionRegistry};

use super::registry::ContributionRegistry;

/// 扁平树节点（供 `TreeItem` 等消费方递归组装）
#[derive(Debug, Clone)]
pub struct ContributionTreeNode {
    pub id: SharedString,
    pub name: SharedString,
    pub order: i32,
    pub parent_id: Option<SharedString>,
    pub children: Vec<ContributionTreeNode>,
}

struct FlatNode {
    id: SharedString,
    name: SharedString,
    order: i32,
    parent_id: Option<SharedString>,
}

/// 读取 host 下贡献并组装为森林（多棵根节点，`parent_id == None` 为根）
pub fn build_contribution_tree(
    registry: &ContributionRegistry,
    host_id: &str,
) -> Vec<ContributionTreeNode> {
    let Some(host) = registry.host(host_id) else {
        return Vec::new();
    };

    let flats: Vec<FlatNode> = host.entries().iter().map(flat_from_entry).collect();
    let mut by_parent: HashMap<Option<String>, Vec<&FlatNode>> = HashMap::new();
    for node in &flats {
        let key = node.parent_id.as_ref().map(|p| p.to_string());
        by_parent.entry(key).or_default().push(node);
    }
    build_children(None, &by_parent)
}

fn build_children(
    parent_id: Option<&str>,
    by_parent: &HashMap<Option<String>, Vec<&FlatNode>>,
) -> Vec<ContributionTreeNode> {
    let key = parent_id.map(|s| s.to_string());
    let mut siblings = by_parent.get(&key).cloned().unwrap_or_default();
    siblings.sort_by_key(|n| n.order);
    siblings
        .into_iter()
        .map(|n| ContributionTreeNode {
            id: n.id.clone(),
            name: n.name.clone(),
            order: n.order,
            parent_id: n.parent_id.clone(),
            children: build_children(Some(n.id.as_ref()), by_parent),
        })
        .collect()
}

fn flat_from_entry(entry: &ContributedEntry) -> FlatNode {
    let c = entry.contribution.as_ref();
    FlatNode {
        id: SharedString::from(c.id()),
        name: c.name(),
        order: entry.options.order,
        parent_id: entry.options.parent_id.clone(),
    }
}

impl ContributionTreeNode {
    /// 深度优先展平为 (id, name, depth) 列表（调试 / 简单渲染）
    pub fn flatten_with_depth(
        &self,
        depth: usize,
        out: &mut Vec<(SharedString, SharedString, usize)>,
    ) {
        out.push((self.id.clone(), self.name.clone(), depth));
        for child in &self.children {
            child.flatten_with_depth(depth + 1, out);
        }
    }
}

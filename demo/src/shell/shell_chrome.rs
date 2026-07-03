//! 贡献条目投影：将 host 受理的贡献条目投影为 Shell 控件数据。
//!
//! 单一存储桶 `Vec<ContribEntry>`（host 入口处 `unwrap_or_default`），
//! 投影函数按 slot/order/parent_id/group 分组，经 `as_command()`/`as_visual()`
//! 能力查询区分贡献类型，供 MainWindow/ActivityPanel 在 `on_loaded` / `refresh_*` 中调用。

use std::collections::HashMap;
use std::sync::Arc;

use rml_core::command::CommandAbilityExt;
use rml_core::contribution::{ContributionOptions, IContribution, VisualAbilityExt};
use rml_ui::{
    IMenuItem, IStatusBarItem, MenuItem, StatusBarAlign, StatusBarItem, TreeItem,
    VisualActivityPanel,
};

pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions);

fn contribs_in_slot<'a>(entries: &'a [ContribEntry], slot: &str) -> Vec<&'a ContribEntry> {
    entries
        .iter()
        .filter(|(_, o)| o.effective_slot() == Some(slot))
        .collect()
}

pub fn map_status_items(entries: &[ContribEntry]) -> Vec<Arc<dyn IStatusBarItem>> {
    let mut items = contribs_in_slot(entries, "status");
    items.sort_by_key(|(_, o)| o.order);
    items
        .into_iter()
        .map(|(c, o)| {
            let align = match o.properties.get("align").map(|s| s.as_ref()) {
                Some("right") => StatusBarAlign::Right,
                _ => StatusBarAlign::Left,
            };
            StatusBarItem::new(c.name())
                .align(align)
                .into_arc()
        })
        .collect()
}

/// 菜单树节点（合并 submenu root 与 leaf command）
struct MenuNode {
    id: String,
    name: gpui::SharedString,
    order: i32,
    parent_id: Option<String>,
    /// 叶子节点携带贡献引用（submenu root 为 None）
    contribution: Option<Arc<dyn IContribution>>,
}

pub fn map_menu_items(entries: &[ContribEntry]) -> Vec<Arc<dyn IMenuItem>> {
    let mut all: Vec<MenuNode> = Vec::new();

    for (c, o) in entries.iter().filter(|(_, o)| o.effective_slot() == Some("menu")) {
        all.push(MenuNode {
            id: c.id().to_string(),
            name: c.name(),
            order: o.order,
            parent_id: o.parent_id.as_ref().map(|s| s.to_string()),
            // 通过 as_command() 判定是否为叶子命令
            contribution: c.as_command().map(|_| c.clone()),
        });
    }

    // 按 parent_id 建树
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

pub fn map_case_tree_items(entries: &[ContribEntry]) -> Vec<TreeItem> {
    use rml_core::i18n::t_static;
    let entries = entries
        .iter()
        .filter(|(c, o)| o.effective_slot() == Some("case") && c.as_visual().is_some())
        .collect::<Vec<_>>();

    let mut by_group: HashMap<Option<String>, Vec<&ContribEntry>> = HashMap::new();
    for e in &entries {
        by_group
            .entry(e.1.group.as_ref().map(|s| s.to_string()))
            .or_default()
            .push(e);
    }

    let mut groups: Vec<(Option<String>, i32)> = by_group
        .iter()
        .map(|(g, items)| {
            (
                g.clone(),
                items.iter().map(|(_, o)| o.order).min().unwrap_or(0),
            )
        })
        .collect();
    groups.sort_by_key(|(_, o)| *o);

    let mut result: Vec<TreeItem> = Vec::new();
    for (group, _) in groups {
        let mut siblings = by_group.get(&group).cloned().unwrap_or_default();
        siblings.sort_by_key(|(_, o)| o.order);

        match group {
            Some(g) => {
                let group_id = format!("group.{}", g);
                let group_name = t_static(&format!("tree.group.{}", g));
                let mut item = TreeItem::new(group_id, group_name).expanded(true);
                for (c, _) in siblings {
                    let child_item = TreeItem::new(c.id(), c.name());
                    item = item.child(child_item);
                }
                result.push(item);
            }
            None => {
                for (c, _) in siblings {
                    result.push(TreeItem::new(c.id(), c.name()));
                }
            }
        }
    }
    result
}

/// 从 `slot="activity"` 的视觉贡献构造活动栏面板列表。
///
/// 每个视觉贡献经 `VisualActivityPanel` 适配为 `IActivityPanel`，
/// `panel()` 委托给 `IVisualContribution::render`（经框架实体缓存复用 Entity）。
pub fn build_activity_panels_from(
    entries: &[ContribEntry],
) -> Vec<Arc<dyn rml_ui::IActivityPanel>> {
    let mut panels = entries
        .iter()
        .filter(|(c, o)| o.effective_slot() == Some("activity") && c.as_visual().is_some())
        .collect::<Vec<_>>();
    panels.sort_by_key(|(_, o)| o.order);
    panels
        .into_iter()
        .filter_map(|(c, _)| {
            VisualActivityPanel::new(c.clone())
                .map(|p| Arc::new(p) as Arc<dyn rml_ui::IActivityPanel>)
        })
        .collect()
}

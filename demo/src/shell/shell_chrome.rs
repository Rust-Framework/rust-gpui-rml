//! 贡献条目投影：将 host 受理的贡献条目投影为 Shell 控件数据。
//!
//! 贡献分两类存储：
//! - **非视觉**（menu/status）：`Vec<(Arc<dyn IContribution>, ContributionOptions)>`
//! - **视觉**（case/activity）：`Vec<(Arc<dyn IVisualContribution>, ContributionOptions)>`
//!
//! 投影函数按 slot/order/parent_id/group 分组，供 MainWindow/ActivityPanel 在
//! `on_loaded` / `refresh_*` 中调用。

use std::collections::HashMap;
use std::sync::Arc;

use rml_core::command::ICommand;
use rml_core::contribution::{ContributionOptions, IContribution, IVisualContribution};
use rml_ui::{
    ActivityPanels, MenuItem, MenuItems, StatusBarAlign, StatusBarItem, StatusBarItems,
    TreeItem, VisualActivityPanel,
};

pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions);
pub type VisualEntry = (Arc<dyn IVisualContribution>, ContributionOptions);

fn contribs_in_slot<'a>(entries: &'a [ContribEntry], slot: &str) -> Vec<&'a ContribEntry> {
    entries
        .iter()
        .filter(|(_, o)| o.effective_slot() == Some(slot))
        .collect()
}

fn visuals_in_slot<'a>(entries: &'a [VisualEntry], slot: &str) -> Vec<&'a VisualEntry> {
    entries
        .iter()
        .filter(|(_, o)| o.effective_slot() == Some(slot))
        .collect()
}

pub fn map_status_items(entries: &[ContribEntry]) -> StatusBarItems {
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

pub fn map_menu_items(
    entries: &[ContribEntry],
    commands: &HashMap<String, Arc<dyn ICommand>>,
) -> MenuItems {
    let entries = contribs_in_slot(entries, "menu");
    let mut by_parent: HashMap<Option<String>, Vec<&ContribEntry>> = HashMap::new();
    for e in &entries {
        by_parent
            .entry(e.1.parent_id.as_ref().map(|s| s.to_string()))
            .or_default()
            .push(e);
    }

    fn build_children(
        parent_id: Option<&str>,
        by_parent: &HashMap<Option<String>, Vec<&ContribEntry>>,
        commands: &HashMap<String, Arc<dyn ICommand>>,
    ) -> MenuItems {
        let key = parent_id.map(|s| s.to_string());
        let mut siblings = by_parent.get(&key).cloned().unwrap_or_default();
        siblings.sort_by_key(|(_, o)| o.order);
        siblings
            .into_iter()
            .map(|(c, _)| {
                let id = c.id();
                let mut item = MenuItem::new(c.name());
                if let Some(cmd) = commands.get(id) {
                    item = item.command(cmd.clone());
                }
                let children = build_children(Some(id), by_parent, commands);
                if !children.is_empty() {
                    item = item.children(children);
                }
                item.into_arc()
            })
            .collect()
    }

    build_children(None, &by_parent, commands)
}

pub fn map_case_tree_items(entries: &[VisualEntry]) -> Vec<TreeItem> {
    use rml_core::i18n::t_static;
    let entries = visuals_in_slot(entries, "case");

    let mut by_group: HashMap<Option<String>, Vec<&VisualEntry>> = HashMap::new();
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

/// 从 `slot="activity"` 的视觉贡献构造 `ActivityPanels`。
///
/// 每个视觉贡献经 `VisualActivityPanel` 适配为 `IActivityPanel`，
/// `panel()` 委托给 `IVisualContribution::render`（经框架实体缓存复用 Entity）。
pub fn build_activity_panels_from(entries: &[VisualEntry]) -> ActivityPanels {
    let mut panels = visuals_in_slot(entries, "activity");
    panels.sort_by_key(|(_, o)| o.order);
    panels
        .into_iter()
        .filter_map(|(c, _)| {
            VisualActivityPanel::new(c.clone())
                .map(|p| Arc::new(p) as Arc<dyn rml_ui::IActivityPanel>)
        })
        .collect()
}

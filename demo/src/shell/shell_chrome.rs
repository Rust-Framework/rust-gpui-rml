//! 贡献条目投影：将 host 受理的 `ContributionEntry` 投影为 Shell 控件数据。
//!
//! `ContributionEntry` 由框架定义（`rml_core::contribution`），`#[contributehost]` 宏
//! 自动注入 `entries: ObservableVec<ContributionEntry>` 字段。投影函数按
//! slot/order/parent_id 分组，供 MainWindow/ActivityPanel 在 `host_on_loaded` 中调用。

use std::collections::HashMap;
use std::sync::Arc;

use rml_core::command::ICommand;
use rml_core::contribution::ContributionEntry;
use rml_ui::{MenuItem, MenuItems, StatusBarAlign, StatusBarItem, StatusBarItems, TreeItem};

fn entries_in_slot<'a>(entries: &'a [ContributionEntry], slot: &str) -> Vec<&'a ContributionEntry> {
    entries
        .iter()
        .filter(|e| e.options.effective_slot() == Some(slot))
        .collect()
}

pub fn map_status_items(entries: &[ContributionEntry]) -> StatusBarItems {
    let mut items = entries_in_slot(entries, "status");
    items.sort_by_key(|e| e.options.order);
    items
        .into_iter()
        .map(|e| {
            let align = match e.options.properties.get("align").map(|s| s.as_ref()) {
                Some("right") => StatusBarAlign::Right,
                _ => StatusBarAlign::Left,
            };
            StatusBarItem::new(e.contribution.name())
                .align(align)
                .into_arc()
        })
        .collect()
}

pub fn map_menu_items(
    entries: &[ContributionEntry],
    commands: &HashMap<String, Arc<dyn ICommand>>,
) -> MenuItems {
    let entries = entries_in_slot(entries, "menu");
    let mut by_parent: HashMap<Option<String>, Vec<&ContributionEntry>> = HashMap::new();
    for e in &entries {
        by_parent
            .entry(e.options.parent_id.as_ref().map(|s| s.to_string()))
            .or_default()
            .push(e);
    }

    fn build_children(
        parent_id: Option<&str>,
        by_parent: &HashMap<Option<String>, Vec<&ContributionEntry>>,
        commands: &HashMap<String, Arc<dyn ICommand>>,
    ) -> MenuItems {
        let key = parent_id.map(|s| s.to_string());
        let mut siblings = by_parent.get(&key).cloned().unwrap_or_default();
        siblings.sort_by_key(|e| e.options.order);
        siblings
            .into_iter()
            .map(|e| {
                let id = e.contribution.id();
                let mut item = MenuItem::new(e.contribution.name());
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

pub fn map_case_tree_items(entries: &[ContributionEntry]) -> Vec<TreeItem> {
    use rml_core::i18n::t_static;
    let entries = entries_in_slot(entries, "case");

    let mut by_group: HashMap<Option<String>, Vec<&ContributionEntry>> = HashMap::new();
    for e in &entries {
        by_group
            .entry(e.options.group.as_ref().map(|s| s.to_string()))
            .or_default()
            .push(e);
    }

    let mut groups: Vec<(Option<String>, i32)> = by_group
        .iter()
        .map(|(g, items)| {
            (
                g.clone(),
                items.iter().map(|e| e.options.order).min().unwrap_or(0),
            )
        })
        .collect();
    groups.sort_by_key(|(_, o)| *o);

    let mut result: Vec<TreeItem> = Vec::new();
    for (group, _) in groups {
        let mut siblings = by_group.get(&group).cloned().unwrap_or_default();
        siblings.sort_by_key(|e| e.options.order);

        match group {
            Some(g) => {
                let group_id = format!("group.{}", g);
                let group_name = t_static(&format!("tree.group.{}", g));
                let mut item = TreeItem::new(group_id, group_name).expanded(true);
                for child in siblings {
                    let child_item =
                        TreeItem::new(child.contribution.id(), child.contribution.name());
                    item = item.child(child_item);
                }
                result.push(item);
            }
            None => {
                for e in siblings {
                    result.push(TreeItem::new(e.contribution.id(), e.contribution.name()));
                }
            }
        }
    }
    result
}

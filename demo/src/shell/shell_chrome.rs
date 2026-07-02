//! 贡献条目投影：将 host 受理的 `ContributedEntry` 投影为 Shell 控件数据。
//!
//! Activity 面板由 `MainWindow` 直接创建 Entity（非贡献注册）——
//! host 在 `on_loaded` 中 `cx.new(|_| ActivityPanel::default())` 后包装为 `IActivityPanel`。

use std::collections::HashMap;
use std::sync::Arc;

use rml_core::command::ICommand;
use rml_core::contribution::{ContributionOptions, IContribution};
use rml_ui::{MenuItem, MenuItems, StatusBarAlign, StatusBarItem, StatusBarItems, TreeItem};

/// host 自管的贡献条目——存储 `Arc<dyn IContribution>` 与 options。
/// 由 `MainWindow::add` 受理时构造，`map_*` 投影函数按 slot/order/parent_id 分组。
#[derive(Clone)]
pub struct ContributedEntry {
    pub contribution: Arc<dyn IContribution>,
    pub options: ContributionOptions,
}

fn entries_in_slot<'a>(entries: &'a [ContributedEntry], slot: &str) -> Vec<&'a ContributedEntry> {
    entries
        .iter()
        .filter(|e| e.options.effective_slot() == Some(slot))
        .collect()
}

pub fn map_status_items(entries: &[ContributedEntry]) -> StatusBarItems {
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
    entries: &[ContributedEntry],
    commands: &HashMap<String, Arc<dyn ICommand>>,
) -> MenuItems {
    let entries = entries_in_slot(entries, "menu");
    let mut by_parent: HashMap<Option<String>, Vec<&ContributedEntry>> = HashMap::new();
    for e in &entries {
        by_parent
            .entry(e.options.parent_id.as_ref().map(|s| s.to_string()))
            .or_default()
            .push(e);
    }

    fn build_children(
        parent_id: Option<&str>,
        by_parent: &HashMap<Option<String>, Vec<&ContributedEntry>>,
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

pub fn map_case_tree_items(entries: &[ContributedEntry]) -> Vec<TreeItem> {
    use rml_core::i18n::t_static;
    let entries = entries_in_slot(entries, "case");

    let mut by_group: HashMap<Option<String>, Vec<&ContributedEntry>> = HashMap::new();
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

pub struct ShellChromeBindings {
    pub status_items: StatusBarItems,
    pub menu_items: MenuItems,
}

pub fn map_shell_chrome(
    entries: &[ContributedEntry],
    menu_commands: &HashMap<String, Arc<dyn ICommand>>,
) -> ShellChromeBindings {
    ShellChromeBindings {
        status_items: map_status_items(entries),
        menu_items: map_menu_items(entries, menu_commands),
    }
}

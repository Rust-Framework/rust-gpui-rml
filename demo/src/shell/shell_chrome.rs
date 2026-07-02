//! 贡献 registry -> Shell 控件数据的应用层映射（menu / status / case 树）。
//!
//! Activity 面板由 `rml_app::contribution::map_activity_panels` 从视觉贡献组装：
//! icon/name 为贡献元数据，展开内容由组件贡献（`#[contribute]` + `#[component]`）渲染。

use std::collections::HashMap;
use std::sync::Arc;

use gpui::Context;
use rml_app::contribution::{contribution_entries, map_activity_panels};
use rml_core::command::ICommand;
use rml_core::contribution::ContributedEntry;
use rml_ui::{
    ActivityPanels, MenuItem, MenuItems, StatusBarAlign, StatusBarItem, StatusBarItems, TreeItem,
};

fn entries_in_slot<'a, C>(
    cx: &'a Context<C>,
    host_id: &str,
    slot: &str,
) -> Vec<&'a ContributedEntry> {
    contribution_entries(host_id, cx)
        .iter()
        .filter(|e| e.options.effective_slot() == Some(slot))
        .collect()
}

pub fn map_status_items<C>(host_id: &str, cx: &Context<C>) -> StatusBarItems {
    let mut entries = entries_in_slot(cx, host_id, "status");
    entries.sort_by_key(|e| e.options.order);
    entries
        .into_iter()
        .map(|e| {
            let align = match e
                .options
                .properties
                .get("align")
                .map(|s| s.as_ref())
            {
                Some("right") => StatusBarAlign::Right,
                _ => StatusBarAlign::Left,
            };
            StatusBarItem::new(e.contribution.name())
                .align(align)
                .into_arc()
        })
        .collect()
}

pub fn map_menu_items<C>(
    host_id: &str,
    cx: &Context<C>,
    commands: &HashMap<String, Arc<dyn ICommand>>,
) -> MenuItems {
    let entries = entries_in_slot(cx, host_id, "menu");
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

pub fn map_case_tree_items<C>(host_id: &str, cx: &Context<C>) -> Vec<TreeItem> {
    use rml_core::i18n::t_static;
    let entries = entries_in_slot(cx, host_id, "case");

    // 按 group 分组（None 为零散顶层节点）
    let mut by_group: HashMap<Option<String>, Vec<&ContributedEntry>> = HashMap::new();
    for e in &entries {
        by_group
            .entry(e.options.group.as_ref().map(|s| s.to_string()))
            .or_default()
            .push(e);
    }

    // group 的代表 order = 组内最小 case order，用于分类 folder 顶层排序
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
    pub activity_panels: ActivityPanels,
    pub status_items: StatusBarItems,
    pub menu_items: MenuItems,
}

pub fn map_shell_chrome<C>(
    host_id: &str,
    cx: &Context<C>,
    menu_commands: &HashMap<String, Arc<dyn ICommand>>,
) -> ShellChromeBindings {
    ShellChromeBindings {
        activity_panels: map_activity_panels(host_id, cx),
        status_items: map_status_items(host_id, cx),
        menu_items: map_menu_items(host_id, cx, menu_commands),
    }
}

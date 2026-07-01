//! Single-host contribution mapping: `demo.shell` entries by `properties["kind"]`
//! to UI binding data (ActivityPanels / StatusBarItems / MenuItems / TreeItem).

use std::collections::HashMap;
use std::sync::Arc;

use gpui::SharedString;
use rml_app::contribution::contribution_entries;
use rml_core::command::ICommand;
use rml_core::contribution::ContributedEntry;
use rml_ui::{
    IconName, MenuItem, MenuItems, StatusBarAlign, StatusBarItem, StatusBarItems, TreeItem,
};

/// Main window host id (same as `MainWindow::ID`; literal avoids module cycle).
pub const SHELL_HOST: &str = "demo.shell";

pub const KIND_MENU: &str = "menu";
pub const KIND_ACTIVITY: &str = "activity";
pub const KIND_STATUS: &str = "status";
pub const KIND_CASE: &str = "case";

pub(super) fn kind_of(entry: &ContributedEntry) -> Option<&str> {
    entry.options.properties.get("kind").map(|s| s.as_ref())
}

pub(super) fn icon_from_name(name: &str) -> IconName {
    match name {
        "BookOpen" => IconName::BookOpen,
        "Settings" => IconName::Settings,
        "Frame" => IconName::Frame,
        _ => IconName::Frame,
    }
}

pub(super) fn host_entries<'a, C>(cx: &'a gpui::Context<C>, host_id: &str) -> Vec<&'a ContributedEntry> {
    contribution_entries(host_id, cx).iter().collect()
}

/// host -> StatusBarItems (kind=status)
pub fn build_status_items<C>(cx: &gpui::Context<C>) -> StatusBarItems {
    let mut entries: Vec<&ContributedEntry> = host_entries(cx, SHELL_HOST)
        .into_iter()
        .filter(|e| kind_of(e) == Some(KIND_STATUS))
        .collect();
    entries.sort_by_key(|e| e.options.order);

    entries
        .into_iter()
        .map(|e| {
            let align = match e.options.placement {
                Some(rml_core::contribution::VisualPlacement::Right) => StatusBarAlign::Right,
                _ => StatusBarAlign::Left,
            };
            StatusBarItem::new(e.contribution.name())
                .align(align)
                .into_arc()
        })
        .collect()
}

/// host -> MenuItems (kind=menu); commands from side table by id
pub fn build_menu_items<C>(
    cx: &gpui::Context<C>,
    commands: &HashMap<String, Arc<dyn ICommand>>,
) -> MenuItems {
    let entries: Vec<&ContributedEntry> = host_entries(cx, SHELL_HOST)
        .into_iter()
        .filter(|e| kind_of(e) == Some(KIND_MENU))
        .collect();

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

/// host -> TreeItem tree (kind=case, hierarchical via parent_id)
pub fn build_case_tree_items<C>(cx: &gpui::Context<C>) -> Vec<TreeItem> {
    use std::collections::HashMap as Map;

    struct Flat {
        id: String,
        name: SharedString,
        order: i32,
        parent_id: Option<String>,
    }

    let flats: Vec<Flat> = host_entries(cx, SHELL_HOST)
        .into_iter()
        .filter(|e| kind_of(e) == Some(KIND_CASE))
        .map(|e| Flat {
            id: e.contribution.id().to_string(),
            name: e.contribution.name(),
            order: e.options.order,
            parent_id: e.options.parent_id.as_ref().map(|s| s.to_string()),
        })
        .collect();

    let mut by_parent: Map<Option<String>, Vec<&Flat>> = HashMap::new();
    for node in &flats {
        by_parent
            .entry(node.parent_id.clone())
            .or_default()
            .push(node);
    }

    fn build_children(
        parent_id: Option<&str>,
        by_parent: &HashMap<Option<String>, Vec<&Flat>>,
    ) -> Vec<TreeItem> {
        let key = parent_id.map(|s| s.to_string());
        let mut siblings = by_parent.get(&key).cloned().unwrap_or_default();
        siblings.sort_by_key(|n| n.order);
        siblings
            .into_iter()
            .map(|n| {
                let mut item = TreeItem::new(n.id.clone(), n.name.clone());
                let children = build_children(Some(n.id.as_str()), by_parent);
                if !children.is_empty() {
                    item = item.expanded(true);
                    for child in children {
                        item = item.child(child);
                    }
                }
                item
            })
            .collect()
    }

    build_children(None, &by_parent)
}

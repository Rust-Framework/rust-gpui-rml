//! Demo Shell：把 registry 条目映射为 ActivityBar / Menu / StatusBar / Tree 数据。
//!
//! 这是**应用层参考实现**，不是框架契约。无 UI 的 host（如数据库提供程序）只需
//! `contribution_entries(host_id, cx)` 读取能力扩展，无需此类映射。

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Context, SharedString};
use gpui_component::IconName;
use rml_app::contribution::{contribution_entries, render_contribution_visual};
use rml_core::command::ICommand;
use rml_core::contribution::{ContributedEntry, VisualRenderer};
use rml_ui::{
    ActivityPanels, IActivityPanel, MenuItem, MenuItems, StatusBarAlign, StatusBarItem,
    StatusBarItems, TreeItem,
};

fn icon_from_contribution(name: &str) -> IconName {
    match name {
        "BookOpen" => IconName::BookOpen,
        "Settings" => IconName::Settings,
        "Frame" => IconName::Frame,
        _ => IconName::Frame,
    }
}

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

/// ActivityBar 面板适配：把 `visual` 贡献条目包装为 `IActivityPanel`。
struct ActivityPanelFromContribution {
    id: SharedString,
    icon: IconName,
    title: SharedString,
    visual: Option<VisualRenderer>,
}

impl IActivityPanel for ActivityPanelFromContribution {
    fn id(&self) -> SharedString {
        self.id.clone()
    }

    fn icon(&self) -> IconName {
        self.icon.clone()
    }

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn is_activated(&self) -> bool {
        false
    }

    fn panel(
        &self,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Option<gpui::AnyElement> {
        self.visual
            .as_ref()
            .and_then(|v| render_contribution_visual(v, window, cx))
    }
}

pub fn map_activity_panels<C>(host_id: &str, cx: &Context<C>) -> ActivityPanels {
    let mut entries = entries_in_slot(cx, host_id, "activity");
    entries.sort_by_key(|e| e.options.order);
    entries
        .into_iter()
        .map(|e| {
            let id = e.contribution.id();
            let icon = e
                .contribution
                .icon()
                .map(|s| icon_from_contribution(s.as_ref()))
                .unwrap_or(IconName::Frame);
            Arc::new(ActivityPanelFromContribution {
                id: id.into(),
                icon,
                title: e.contribution.name(),
                visual: e.visual.clone(),
            }) as Arc<dyn IActivityPanel>
        })
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
    let entries = entries_in_slot(cx, host_id, "case");
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
    ) -> Vec<TreeItem> {
        let key = parent_id.map(|s| s.to_string());
        let mut siblings = by_parent.get(&key).cloned().unwrap_or_default();
        siblings.sort_by_key(|e| e.options.order);
        siblings
            .into_iter()
            .map(|e| {
                let id = e.contribution.id();
                let mut item = TreeItem::new(id, e.contribution.name());
                let children = build_children(Some(id), by_parent);
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

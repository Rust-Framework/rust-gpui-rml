//! 贡献 registry → Shell 控件数据的**应用层映射**（框架只存元数据，不做 UI 组装）。
//!
//! 各扩展模块 `#[contribute]` 注册条目后，Host 在 `refresh_bindings` 里调用
//! `map_shell_chrome` 按 `slot`/`parent_id` 拼出 `MenuItems`、`ActivityPanels` 等。
//! 这是预期的一步胶水，不是框架缺陷；若 Shell 完全静态，可删掉本文件改手写 VM 字段。

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled};
use gpui_component::IconName;
use rml_app::contribution::contribution_entries;
use rml_core::command::ICommand;
use rml_core::contribution::ContributedEntry;
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

/// Activity 面板：直接挂载 GPUI Entity（可靠路径，避免 visual 缓存时序问题）
struct EntityActivityPanel<E: Render + 'static> {
    id: SharedString,
    icon: IconName,
    title: SharedString,
    entity: Entity<E>,
}

impl<E: Render + 'static> IActivityPanel for EntityActivityPanel<E> {
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
        _window: &mut gpui::Window,
        _cx: &mut gpui::App,
    ) -> Option<gpui::AnyElement> {
        Some(
            gpui::div()
                .size_full()
                .child(self.entity.clone())
                .into_any_element(),
        )
    }
}

/// 将 activity slot 元数据与 ViewModel 持有的 Entity 合并为 ActivityPanels
pub fn map_activity_panels<E, C>(
    host_id: &str,
    cx: &Context<C>,
    panel_entities: &HashMap<String, Entity<E>>,
) -> ActivityPanels
where
    E: Render + 'static,
{
    let mut entries = entries_in_slot(cx, host_id, "activity");
    entries.sort_by_key(|e| e.options.order);
    entries
        .into_iter()
        .filter_map(|e| {
            let id = e.contribution.id();
            let entity = panel_entities.get(id)?;
            let icon = e
                .contribution
                .icon()
                .map(|s| icon_from_contribution(s.as_ref()))
                .unwrap_or(IconName::Frame);
            Some(
                Arc::new(EntityActivityPanel {
                    id: id.into(),
                    icon,
                    title: e.contribution.name(),
                    entity: entity.clone(),
                }) as Arc<dyn IActivityPanel>,
            )
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

pub fn map_shell_chrome<E, C>(
    host_id: &str,
    cx: &Context<C>,
    menu_commands: &HashMap<String, Arc<dyn ICommand>>,
    panel_entities: &HashMap<String, Entity<E>>,
) -> ShellChromeBindings
where
    E: Render + 'static,
{
    ShellChromeBindings {
        activity_panels: map_activity_panels(host_id, cx, panel_entities),
        status_items: map_status_items(host_id, cx),
        menu_items: map_menu_items(host_id, cx, menu_commands),
    }
}

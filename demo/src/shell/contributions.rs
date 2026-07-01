//! 单 Host 贡献映射 —— 将 `demo.shell` host 中的贡献按 `properties["kind"]`
//! 分类，映射为 UI 绑定数据（ActivityPanels / StatusBarItems / MenuItems / TreeItem）。
//!
//! 同时提供程序化注册辅助：菜单/状态/案例分类节点（纯元数据贡献）。

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{App, BorrowAppContext, SharedString};
use rml_app::contribution::{data_registerable, ContributionRegistryGlobal, Registerable};
use rml_core::command::ICommand;
use rml_core::contribution::{
    ContributionOptions, ContributedEntry, IContribution, IContributionRegistry,
};
use rml_ui::{
    ActivityPanel, ActivityPanels, IconName, MenuItem, MenuItems, StatusBarAlign, StatusBarItem,
    StatusBarItems, TreeItem,
};

/// 单一 host_id —— MainWindow 管理的所有贡献
pub const SHELL_HOST: &str = "demo.shell";

pub const KIND_MENU: &str = "menu";
pub const KIND_ACTIVITY: &str = "activity";
pub const KIND_STATUS: &str = "status";
pub const KIND_CASE: &str = "case";

fn kind_of(entry: &ContributedEntry) -> Option<&str> {
    entry.options.properties.get("kind").map(|s| s.as_ref())
}

fn icon_from_name(name: &str) -> IconName {
    match name {
        "BookOpen" => IconName::BookOpen,
        "Settings" => IconName::Settings,
        "Frame" => IconName::Frame,
        _ => IconName::Frame,
    }
}

fn host_entries<'a, C>(cx: &'a gpui::Context<C>, host_id: &str) -> Vec<&'a ContributedEntry> {
    let registry = &cx.global::<ContributionRegistryGlobal>().0;
    match registry.host(host_id) {
        Some(host) => host.entries().iter().collect(),
        None => Vec::new(),
    }
}

/// host → ActivityPanels（kind=activity）
pub fn build_activity_panels<C>(cx: &gpui::Context<C>, active_id: &str) -> ActivityPanels {
    let mut entries: Vec<&ContributedEntry> = host_entries(cx, SHELL_HOST)
        .into_iter()
        .filter(|e| kind_of(e) == Some(KIND_ACTIVITY))
        .collect();
    entries.sort_by_key(|e| e.options.order);

    entries
        .into_iter()
        .map(|e| {
            let id = e.contribution.id();
            let icon = e
                .contribution
                .icon()
                .map(|s| icon_from_name(s.as_ref()))
                .unwrap_or(IconName::Frame);
            ActivityPanel::new(id, icon, e.contribution.name())
                .active(active_id == id)
                .into_arc()
        })
        .collect()
}

/// host → StatusBarItems（kind=status）
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

/// host → MenuItems（kind=menu），命令从 `commands` 侧表按 id 查找挂接
///
/// 支持按 `parent_id` 组装子菜单层级：顶层菜单项（parent_id=None）作为菜单栏入口，
/// 其子项（parent_id=父菜单 id）作为下拉菜单内容。
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

/// host → TreeItem 树（kind=case，按 `parent_id` 层级组装）
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

// ─── 纯元数据贡献结构（程序化注册） ───────────────────────────────

#[derive(Clone)]
struct TextContribution {
    id: &'static str,
    name_key: &'static str,
}

impl IContribution for TextContribution {
    fn id(&self) -> &str {
        self.id
    }
    fn name(&self) -> SharedString {
        rml_core::i18n::t_static(self.name_key).into()
    }
    fn description(&self) -> SharedString {
        SharedString::default()
    }
    fn icon(&self) -> Option<SharedString> {
        None
    }
}

impl Registerable for TextContribution {
    fn into_entry(
        contribution: Arc<Self>,
        options: ContributionOptions,
    ) -> ContributedEntry {
        data_registerable(contribution, options)
    }
}

fn register_text(cx: &mut App, id: &'static str, name_key: &'static str, kind: &str, order: i32) {
    let contribution = Arc::new(TextContribution { id, name_key });
    let options = ContributionOptions::new()
        .property("kind", kind)
        .order(order);
    cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
        global.0.register(SHELL_HOST, contribution, options, cx);
    });
}

/// 注册菜单元数据贡献（kind=menu）。命令由 MainWindow 在 `menu_commands` 侧表维护。
pub fn register_menu_entry(cx: &mut App, id: &'static str, name_key: &'static str, order: i32) {
    register_text(cx, id, name_key, KIND_MENU, order);
}

/// 注册带父菜单的菜单元数据贡献（kind=menu + parent_id）。
/// 顶层菜单用 `parent_id=None`，子菜单项用 `parent_id=Some(父菜单 id)`。
pub fn register_menu_entry_with_parent(
    cx: &mut App,
    id: &'static str,
    name_key: &'static str,
    parent_id: Option<&'static str>,
    order: i32,
) {
    let contribution = Arc::new(TextContribution { id, name_key });
    let mut options = ContributionOptions::new()
        .property("kind", KIND_MENU)
        .order(order);
    if let Some(p) = parent_id {
        options = options.parent_id(p);
    }
    cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
        global.0.register(SHELL_HOST, contribution, options, cx);
    });
}

/// 注册状态栏文本贡献（kind=status）
pub fn register_status_entry(cx: &mut App, id: &'static str, name_key: &'static str, order: i32) {
    register_text(cx, id, name_key, KIND_STATUS, order);
}

/// 注册案例分类节点（kind=case，parent_id=None，树根）
pub fn register_case_categories(cx: &mut App) {
    register_case_node(cx, "cat.binding", "tree.cat.binding", None, 0);
    register_case_node(cx, "cat.components", "tree.cat.components", None, 10);
    register_case_node(cx, "cat.menu", "tree.cat.menu", None, 15);
    register_case_node(cx, "cat.i18n", "tree.cat.i18n", None, 20);
}

fn register_case_node(
    cx: &mut App,
    id: &'static str,
    name_key: &'static str,
    parent_id: Option<&'static str>,
    order: i32,
) {
    let contribution = Arc::new(TextContribution { id, name_key });
    let mut options = ContributionOptions::new().property("kind", KIND_CASE).order(order);
    if let Some(p) = parent_id {
        options = options.parent_id(p);
    }
    cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
        global.0.register(SHELL_HOST, contribution, options, cx);
    });
}

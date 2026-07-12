//! 菜单视图模型 —— 从贡献条目解包的类型化树结构。
//!
//! 镜像 `StatusViewModel` 模式：`MenuViewModel::from_contribution` 按 `kind="menu"` 过滤，
//! `build_menu_view_models` 按 `parent_id` 组织树、按 `order` 排序。
//! 标签经 `contribution.name()` 动态获取，反映当前 locale。
//! 叶子命令经 `contribution.as_command()` 提取，闭包内重新查询以保持借用安全。

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Context, SharedString, Window};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use rml_core::command::{CallContext, CommandAbilityExt};
use rml_core::contribution::{ContributionOptions, IContribution};

use crate::shell::status_view_model::ContribEntry;

/// 贡献命令包装器 —— 将 `Arc<dyn IContribution>` 包装为 `ICommand` 实现。
///
/// 用于 RML 声明式绑定 `command={m.command()}`，使菜单项可以直接绑定到贡献的命令能力。
#[derive(Clone)]
pub struct ContributedCommand(pub Arc<dyn IContribution>);

impl ContributedCommand {
    /// 委托到底层贡献的命令能力查询。
    pub fn can_execute(&self, ctx: &mut CallContext) -> bool {
        self.0.as_command().map(|c| c.can_execute(ctx)).unwrap_or(false)
    }

    /// 委托到底层贡献的命令执行。
    pub fn execute(&self, ctx: &mut CallContext) {
        if let Some(cmd) = self.0.as_command() {
            cmd.execute(ctx);
        }
    }
}

#[derive(Clone)]
pub struct MenuViewModel {
    pub id: SharedString,
    pub parent_id: Option<SharedString>,
    pub order: i32,
    pub contribution: Arc<dyn IContribution>,
    pub children: Vec<MenuViewModel>,
}

impl MenuViewModel {
    /// 动态标签 — 委托 `contribution.name()`，反映当前 locale。
    pub fn label(&self) -> SharedString {
        self.contribution.name()
    }

    #[allow(dead_code)]
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// 提取命令能力（供 RML `command={m.command()}` 绑定）。
    ///
    /// 若贡献实现了 `ICommand`，返回 `Some(ContributedCommand)`；否则 `None`。
    pub fn command(&self) -> Option<ContributedCommand> {
        if self.contribution.as_command().is_some() {
            Some(ContributedCommand(self.contribution.clone()))
        } else {
            None
        }
    }

    /// 从贡献条目构造；非 menu 槽位返回 `None`。
    pub fn from_contribution(
        c: Arc<dyn IContribution>,
        opts: ContributionOptions,
    ) -> Option<Self> {
        if opts.effective_slot() != Some("menu") {
            return None;
        }
        Some(Self {
            id: c.id().into(),
            parent_id: opts.parent_id,
            order: opts.order,
            contribution: c,
            children: Vec::new(),
        })
    }

    /// 从贡献条目列表构建菜单树（按 `parent_id` 组织，按 `order` 排序）。
    ///
    /// 算法：平铺过滤 → 按 id 建表 → 遍历挂载到父节点 children → 每层排序。
    /// 无法找到父节点的条目（parent_id 指向不存在的 id）视为根节点。
    pub fn build_menu_view_models(entries: &[ContribEntry]) -> Vec<MenuViewModel> {
        let mut nodes: HashMap<SharedString, MenuViewModel> = HashMap::new();
        let mut parent_map: Vec<(SharedString, Option<SharedString>, i32)> = Vec::new();

        for (c, o) in entries {
            if let Some(vm) = Self::from_contribution(c.clone(), o.clone()) {
                parent_map.push((vm.id.clone(), vm.parent_id.clone(), vm.order));
                nodes.insert(vm.id.clone(), vm);
            }
        }

        let mut roots: Vec<SharedString> = Vec::new();
        for (id, parent_id, _) in &parent_map {
            match parent_id {
                Some(pid) if nodes.contains_key(pid) => {
                    let child = match nodes.get(id).cloned() {
                        Some(c) => c,
                        None => continue,
                    };
                    if let Some(parent) = nodes.get_mut(pid) {
                        parent.children.push(child);
                    }
                }
                _ => {
                    roots.push(id.clone());
                }
            }
        }

        let mut result: Vec<MenuViewModel> = roots
            .into_iter()
            .filter_map(|id| nodes.get(&id).cloned())
            .collect();
        result.sort_by_key(|m| m.order);
        for m in &mut result {
            m.children.sort_by_key(|c| c.order);
        }
        result
    }

    /// 递归构建 `PopupMenu`（`dropdown_menu` 闭包内调用）。
    ///
    /// 叶子节点经 `contribution.as_command()` 提取命令；
    /// `contribution` Arc 在闭包外 clone 以满足 `'static` bound，
    /// 闭包内重新调用 `as_command()` 获取 `&dyn ICommand` 借用。
    #[allow(dead_code)]
    pub fn build_popup_menu(
        mut menu: PopupMenu,
        items: &[MenuViewModel],
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        for item in items {
            if item.has_children() {
                let children = item.children.clone();
                let label = item.label();
                menu = menu.submenu(label, window, cx, {
                    let children = children.clone();
                    move |submenu, window, cx| {
                        let submenu = rml_ui::configure_menu_bar_popup(submenu);
                        Self::build_popup_menu(submenu, &children, window, cx)
                    }
                });
            } else {
                let label = item.label();
                let mut pmi = PopupMenuItem::new(label);
                let contrib = item.contribution.clone();
                if contrib.as_command().is_some() {
                    pmi = pmi.on_click(move |_, window, app| {
                        if let Some(cmd) = contrib.as_command() {
                            let mut ctx = CallContext::new(window, app);
                            if cmd.can_execute(&mut ctx) {
                                cmd.execute(&mut ctx);
                            }
                        }
                    });
                }
                menu = menu.item(pmi);
            }
        }
        menu
    }
}

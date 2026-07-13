//! `Tree` —— RML 树视图集成（gpui-component `Tree` 需自定义项渲染，由本 crate 提供默认实现）
//!
//! 树数据由 ViewModel 写入 `TreeState`；声明式 `<TreeNode>` 若将来落地，应由 engine
//! codegen 直译 gpui-component `NativeTree`，本组件仅保留 Stateful / `on_activate` 绑定路径。

use std::rc::Rc;

use gpui::{
    div, px, App, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};
use gpui_component::{
    h_flex,
    list::ListItem,
    tree::{Tree as NativeTree, TreeItem, TreeState},
    Icon, IconName, Sizable as _,
};

type TreeItemHandler = Rc<dyn Fn(TreeItem, &mut Window, &mut App) + 'static>;

/// RML 树视图（`<Tree on-activate="..." on-select="..." />`，状态字段 `tree_state`）
///
/// 接受 `Option<&Entity<TreeState>>`，支持 `on_loaded` 前首次渲染不 panic。
#[derive(IntoElement)]
pub struct Tree {
    state: Option<Entity<TreeState>>,
    on_activate: Option<TreeItemHandler>,
    on_select: Option<TreeItemHandler>,
}

impl Tree {
    pub fn new(state: Option<&Entity<TreeState>>) -> Self {
        Self {
            state: state.cloned(),
            on_activate: None,
            on_select: None,
        }
    }

    /// 叶子节点被点击时触发（分类文件夹不触发）
    pub fn on_activate(
        mut self,
        handler: impl Fn(TreeItem, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_activate = Some(Rc::new(handler));
        self
    }

    pub fn on_activate_rc(mut self, handler: TreeItemHandler) -> Self {
        self.on_activate = Some(handler);
        self
    }

    /// 任意节点被选中时触发（包括分类文件夹）
    pub fn on_select(
        mut self,
        handler: impl Fn(TreeItem, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn on_select_rc(mut self, handler: TreeItemHandler) -> Self {
        self.on_select = Some(handler);
        self
    }
}

impl RenderOnce for Tree {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.state else {
            return div().into_any_element();
        };

        let on_activate = self.on_activate.clone();
        let on_select = self.on_select.clone();
        let state_clone = state.clone();

        NativeTree::new(&state, move |ix, entry, selected, _window, _cx| {
            let icon = if !entry.is_folder() {
                IconName::File
            } else if entry.is_expanded() {
                IconName::FolderOpen
            } else {
                IconName::Folder
            };

            let mut item = ListItem::new(ix)
                .selected(selected)
                .pl(px(16.) * entry.depth() + px(12.))
                .child(
                    h_flex()
                        .gap_2()
                        .child(Icon::new(icon).small())
                        .child(entry.item().label.clone()),
                );

            // 非禁用节点统一挂载 on_click：
            // - on_select 对所有节点触发（含文件夹）
            // - on_activate 仅对叶子节点触发
            if !entry.is_disabled() {
                let is_folder = entry.is_folder();
                let tree_item = entry.item().clone();
                let state = state_clone.clone();
                let on_activate = on_activate.clone();
                let on_select = on_select.clone();
                item = item.on_click(move |_, window, cx| {
                    state.update(cx, |s, cx| {
                        s.set_selected_index(Some(ix), cx);
                    });
                    if let Some(handler) = on_select.as_ref() {
                        handler(tree_item.clone(), window, cx);
                    }
                    if !is_folder {
                        if let Some(handler) = on_activate.as_ref() {
                            handler(tree_item.clone(), window, cx);
                        }
                    }
                });
            }

            item
        })
        .into_any_element()
    }
}

/// ViewModel 侧树节点数据 —— `Send + Sync`，可安全存储在 `#[contribute]` 组件字段中。
///
/// `TreeItem`（gpui-component）内部使用 `Rc<RefCell<...>>`，不满足 `Send + Sync`，
/// 无法存储在 `IContribution` 组件上。`TreeData` 作为声明式数据载体，
/// 在 `TreeState` 构造时经 [`TreeData::to_tree_items`] 转换为 `Vec<TreeItem>`。
///
/// Builder API 与 `TreeItem` 一致：`new` / `child` / `children` / `expanded` / `disabled`。
#[derive(Clone, Default)]
pub struct TreeData {
    pub id: SharedString,
    pub label: SharedString,
    pub children: Vec<TreeData>,
    pub expanded: bool,
    pub disabled: bool,
}

impl TreeData {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            expanded: false,
            disabled: false,
        }
    }

    pub fn child(mut self, child: TreeData) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = TreeData>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 将 `Vec<TreeData>` 转换为 `Vec<TreeItem>`（state_ctor 调用，单线程渲染上下文）。
    pub fn to_tree_items(data: Vec<TreeData>) -> Vec<TreeItem> {
        data.into_iter().map(TreeItem::from).collect()
    }

    /// 是否为文件夹节点（有子节点）。
    pub fn is_folder(&self) -> bool {
        !self.children.is_empty()
    }
}

impl From<TreeData> for TreeItem {
    fn from(data: TreeData) -> Self {
        let mut item = TreeItem::new(data.id, data.label);
        if !data.children.is_empty() {
            item = item.children(data.children.into_iter().map(TreeItem::from));
        }
        if data.expanded {
            item = item.expanded(true);
        }
        if data.disabled {
            item = item.disabled(true);
        }
        item
    }
}

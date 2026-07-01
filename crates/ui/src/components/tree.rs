//! `Tree` —— RML 树视图集成（gpui-component `Tree` 需自定义项渲染，由本 crate 提供默认实现）
//!
//! 树数据由 ViewModel 写入 `TreeState`；声明式 `<TreeNode>` 若将来落地，应由 engine
//! codegen 直译 gpui-component `NativeTree`，本组件仅保留 Stateful / `on_activate` 绑定路径。

use std::rc::Rc;

use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
};
use gpui_component::{
    Icon, IconName, Sizable as _,
    h_flex,
    list::ListItem,
    tree::{Tree as NativeTree, TreeItem, TreeState},
};

/// RML 树视图（`<Tree on_activate="..." />`，状态字段 `tree_state`）
///
/// 接受 `Option<&Entity<TreeState>>`，支持 `on_loaded` 前首次渲染不 panic。
#[derive(IntoElement)]
pub struct Tree {
    state: Option<Entity<TreeState>>,
    on_activate: Option<Rc<dyn Fn(TreeItem, &mut Window, &mut App) + 'static>>,
}

impl Tree {
    pub fn new(state: Option<&Entity<TreeState>>) -> Self {
        Self {
            state: state.cloned(),
            on_activate: None,
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

    pub fn on_activate_rc(
        mut self,
        handler: Rc<dyn Fn(TreeItem, &mut Window, &mut App) + 'static>,
    ) -> Self {
        self.on_activate = Some(handler);
        self
    }
}

impl RenderOnce for Tree {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.state else {
            return div().into_any_element();
        };

        let on_activate = self.on_activate.clone();
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

            if !entry.is_folder() && !entry.is_disabled() {
                if let Some(handler) = on_activate.clone() {
                    let tree_item = entry.item().clone();
                    let state = state_clone.clone();
                    item = item.on_click(move |_, window, cx| {
                        state.update(cx, |s, cx| {
                            s.set_selected_index(Some(ix), cx);
                        });
                        handler(tree_item.clone(), window, cx);
                    });
                }
            }

            item
        })
        .into_any_element()
    }
}

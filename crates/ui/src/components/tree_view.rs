//! TreeView —— gpui-component Tree 的 RML 友好封装
//!
//! 提供官方文档默认的文件夹/文件图标渲染，并支持叶子节点 `on_activate` 回调。

use std::rc::Rc;

use gpui::{
    App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, px,
};
use gpui_component::{
    Icon, IconName, Sizable as _,
    h_flex,
    list::ListItem,
    tree::{Tree, TreeItem, TreeState},
};

/// 带默认项渲染的 Tree 视图（Stateful：`Entity<TreeState>`）
#[derive(IntoElement)]
pub struct TreeView {
    state: Entity<TreeState>,
    on_activate: Option<Rc<dyn Fn(TreeItem, &mut Window, &mut App) + 'static>>,
}

impl TreeView {
    pub fn new(state: &Entity<TreeState>) -> Self {
        Self {
            state: state.clone(),
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

impl RenderOnce for TreeView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let on_activate = self.on_activate.clone();
        let state = self.state.clone();

        Tree::new(&self.state, move |ix, entry, selected, _window, _cx| {
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
                    let state = state.clone();
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
    }
}

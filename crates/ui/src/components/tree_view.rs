//! TreeView —— `<Tree>` 的 RML 集成默认渲染器
//!
//! gpui-component `Tree` 要求调用方提供项渲染闭包；本组件提供案例树场景的默认
//! 文件夹/文件图标与 `on_activate` 事件接线。树数据由 ViewModel 在 Rust 侧写入
//! `TreeState`（非声明式 `<TreeNode>` codegen）。未来若增加声明式树节点，codegen
//! 应直译 `Tree::new`，本组件仅保留 MVVM/Stateful 绑定路径。

use std::rc::Rc;

use gpui::{
    AnyElement, App, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
};
use gpui_component::{
    Icon, IconName, Sizable as _,
    h_flex,
    list::ListItem,
    tree::{Tree, TreeItem, TreeState},
};

/// 带默认项渲染的 Tree 视图（Stateful：`Option<Entity<TreeState>>`）
///
/// 接受 `Option<&Entity<TreeState>>` 以支持 `on_loaded` 前首次渲染不 panic。
#[derive(IntoElement)]
pub struct TreeView {
    state: Option<Entity<TreeState>>,
    on_activate: Option<Rc<dyn Fn(TreeItem, &mut Window, &mut App) + 'static>>,
}

impl TreeView {
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

impl RenderOnce for TreeView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.state else {
            return div().into_any_element();
        };

        let on_activate = self.on_activate.clone();
        let state_clone = state.clone();

        Tree::new(&state, move |ix, entry, selected, _window, _cx| {
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

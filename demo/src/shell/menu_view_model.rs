//! 菜单视图模型 —— 手工构建的类型化树结构。
//!
//! 供 MainWindow.menus 集合持有，RML `<menu-item each={m in menus}>` 直接消费。
//! 菜单不经贡献系统注册（消除 menu_shell_contribs.rs 样板），
//! 叶子节点的 command 字段持有 MainWindow 的 RelayCommand。

use std::sync::Arc;

use gpui::SharedString;
use rml_core::command::ICommand;

#[derive(Clone)]
pub struct MenuViewModel {
    pub id: SharedString,
    pub label: SharedString,
    pub group: Option<SharedString>,
    pub order: i32,
    /// 叶子节点持有 RelayCommand（`Arc<dyn ICommand>`）；submenu root 为 `None`
    pub command: Option<Arc<dyn ICommand>>,
    /// 子菜单（按 order 排序）
    pub children: Vec<MenuViewModel>,
}

impl MenuViewModel {
    /// submenu root 构造（无命令）
    pub fn root(id: &str, label: SharedString, order: i32) -> Self {
        Self {
            id: id.into(),
            label,
            group: None,
            order,
            command: None,
            children: Vec::new(),
        }
    }

    /// 叶子节点构造（带命令）
    pub fn leaf(id: &str, label: SharedString, order: i32, command: Arc<dyn ICommand>) -> Self {
        Self {
            id: id.into(),
            label,
            group: None,
            order,
            command: Some(command),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: MenuViewModel) -> Self {
        self.children.push(child);
        self
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

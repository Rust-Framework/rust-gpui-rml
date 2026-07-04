//! 菜单视图模型 —— 手工构建的类型化树结构。
//!
//! 供 MainWindow.menus 集合持有，`render_menu_bar` 直接消费。
//! 菜单不经贡献系统注册（消除 menu_shell_contribs.rs 样板），
//! 叶子节点的 command 字段持有 MainWindow 的 RelayCommand。

use std::sync::Arc;

use gpui::{Context, SharedString, Window};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use rml_core::command::{CallContext, ICommand};

#[derive(Clone)]
pub struct MenuViewModel {
    pub label: SharedString,
    /// 叶子节点持有 RelayCommand（`Arc<dyn ICommand>`）；submenu root 为 `None`
    pub command: Option<Arc<dyn ICommand>>,
    /// 子菜单
    pub children: Vec<MenuViewModel>,
}

impl MenuViewModel {
    /// submenu root 构造（无命令）
    pub fn root(label: SharedString) -> Self {
        Self {
            label,
            command: None,
            children: Vec::new(),
        }
    }

    /// 叶子节点构造（带命令）
    pub fn leaf(label: SharedString, command: Arc<dyn ICommand>) -> Self {
        Self {
            label,
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

    /// 递归构建 `PopupMenu`（`dropdown_menu` 闭包内调用）。
    ///
    /// `children` 在闭包外 clone 以满足 `'static` bound。
    /// 由 `MainWindow::render_menu_bar()` 的顶层 `dropdown_menu` 闭包启动，
    /// 子菜单经 `PopupMenu::submenu` 递归调用本方法。
    pub fn build_popup_menu(
        mut menu: PopupMenu,
        items: &[MenuViewModel],
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        for item in items {
            if item.has_children() {
                let children = item.children.clone();
                let label = item.label.clone();
                menu = menu.submenu(label, window, cx, {
                    let children = children.clone();
                    move |submenu, window, cx| {
                        let submenu = rml_ui::configure_menu_bar_popup(submenu);
                        Self::build_popup_menu(submenu, &children, window, cx)
                    }
                });
            } else {
                let label = item.label.clone();
                let mut pmi = PopupMenuItem::new(label);
                if let Some(cmd) = item.command.clone() {
                    pmi = pmi.on_click(move |_, window, app| {
                        let mut ctx = CallContext::new(window, app);
                        if cmd.can_execute(&mut ctx) {
                            cmd.execute(&mut ctx);
                        }
                    });
                }
                menu = menu.item(pmi);
            }
        }
        menu
    }
}

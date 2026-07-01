//! Menu —— MVVM 数据绑定的菜单栏组件，封装 gpui-component 的 PopupMenu/DropdownMenu
//!
//! 参照 `activity_bar.rs` 黄金模板：`IMenuItem` trait + `MenuItem` 默认实现 +
//! `MenuItems = Vec<Arc<dyn IMenuItem>>` 类型别名 + `Menu` 容器组件。
//!
//! ViewModel 通过 `#[computed]` 返回 `MenuItems`，在 RML 中 `<menu items={menu_items} />` 绑定。
//! 每个菜单项可携带 `Arc<dyn ICommand>` 命令对象，点击时调用 `execute`。
//!
//! 渲染策略：
//! - 有子菜单的顶层项 → `Button + DropdownMenu → PopupMenu`（支持键盘导航、子菜单递归）
//! - 叶子节点顶层项 → `Button + on_click`（直接执行命令）
//! - PopupMenu 内部项 → `PopupMenuItem`（支持 icon/disabled/checked/submenu/command）

use std::sync::Arc;

use gpui::{App, Context, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window};
use gpui_component::{
    IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    separator::Separator,
    Disableable as _,
};

/// 菜单项接口（object-safe，可存储为 `Arc<dyn IMenuItem>`）
///
/// 实现者可通过 `children()` 返回子菜单项，框架自动递归构建 `PopupMenu`。
pub trait IMenuItem: Send + Sync + 'static {
    fn label(&self) -> gpui::SharedString;
    fn icon(&self) -> Option<IconName> {
        None
    }
    fn disabled(&self) -> bool {
        false
    }
    fn separator(&self) -> bool {
        false
    }
    fn command(&self) -> Option<Arc<dyn rml_core::command::ICommand>> {
        None
    }
    /// 子菜单项：返回 `Some` 时渲染为 DropdownMenu + PopupMenu
    fn children(&self) -> Option<Vec<Arc<dyn IMenuItem>>> {
        None
    }
}

/// 菜单项列表类型别名（供 `#[computed]` 返回类型使用）
pub type MenuItems = Vec<Arc<dyn IMenuItem>>;

/// 菜单项默认实现
pub struct MenuItem {
    label: gpui::SharedString,
    icon: Option<IconName>,
    disabled: bool,
    separator: bool,
    command: Option<Arc<dyn rml_core::command::ICommand>>,
    children: Vec<Arc<dyn IMenuItem>>,
}

impl MenuItem {
    pub fn new(label: impl Into<gpui::SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
            separator: false,
            command: None,
            children: Vec::new(),
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }

    pub fn separator(mut self) -> Self {
        self.separator = true;
        self
    }

    pub fn command(mut self, cmd: Arc<dyn rml_core::command::ICommand>) -> Self {
        self.command = Some(cmd);
        self
    }

    pub fn children(mut self, children: Vec<Arc<dyn IMenuItem>>) -> Self {
        self.children = children;
        self
    }

    pub fn into_arc(self) -> Arc<dyn IMenuItem> {
        Arc::new(self)
    }
}

impl IMenuItem for MenuItem {
    fn label(&self) -> gpui::SharedString {
        self.label.clone()
    }

    fn icon(&self) -> Option<IconName> {
        self.icon.clone()
    }

    fn disabled(&self) -> bool {
        self.disabled
    }

    fn separator(&self) -> bool {
        self.separator
    }

    fn command(&self) -> Option<Arc<dyn rml_core::command::ICommand>> {
        self.command.clone()
    }

    fn children(&self) -> Option<Vec<Arc<dyn IMenuItem>>> {
        if self.children.is_empty() {
            None
        } else {
            Some(self.children.clone())
        }
    }
}

/// Menu 容器组件
///
/// Stateless 构造：`Menu::new(id).items(items)`
/// 通过 `items()` 接收 `MenuItems` 数据绑定。
#[derive(IntoElement)]
pub struct Menu {
    #[allow(dead_code)]
    id: ElementId,
    items: MenuItems,
}

impl Menu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
        }
    }

    pub fn items(mut self, items: MenuItems) -> Self {
        self.items = items;
        self
    }
}

impl RenderOnce for Menu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut bar = h_flex().h_full().items_center();

        for (ix, item) in self.items.iter().enumerate() {
            if item.separator() {
                bar = bar.child(Separator::vertical().h_full());
                continue;
            }

            let label = item.label();
            let disabled = item.disabled();
            let icon = item.icon();
            let children = item.children();
            let command = item.command();

            if let Some(children) = children {
                // 有子菜单：Button + DropdownMenu → PopupMenu
                let mut btn = Button::new(("rml-menu", ix))
                    .label(label)
                    .ghost()
                    .disabled(disabled);

                if let Some(icon) = icon {
                    btn = btn.icon(icon);
                }

                let btn = btn.dropdown_menu(move |menu, window, cx| {
                    build_popup_menu(menu, &children, window, cx)
                });
                bar = bar.child(btn);
            } else {
                // 叶子节点：Button + on_click 直接执行命令
                let mut btn = Button::new(("rml-menu", ix))
                    .label(label)
                    .ghost()
                    .disabled(disabled);

                if let Some(icon) = icon {
                    btn = btn.icon(icon);
                }

                if let Some(cmd) = command {
                    btn = btn.on_click(move |_, _window, cx| cmd.execute(&(), cx));
                }
                bar = bar.child(btn);
            }
        }

        bar
    }
}

/// 递归构建 PopupMenu：从 `IMenuItem` 树生成 gpui-component `PopupMenu`
///
/// 在 `DropdownMenu` 闭包和 `submenu` 闭包中调用，每次调用时 `window`/`cx` 由
/// gpui-component 的弹出层基础设施注入。
fn build_popup_menu(
    mut menu: PopupMenu,
    items: &[Arc<dyn IMenuItem>],
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    for item in items {
        if item.separator() {
            menu = menu.separator();
            continue;
        }

        let label = item.label();
        let disabled = item.disabled();
        let icon = item.icon();
        let children = item.children();
        let command = item.command();

        if let Some(children) = children {
            menu = menu.submenu(label, window, cx, move |submenu, window, cx| {
                build_popup_menu(submenu, &children, window, cx)
            });
        } else {
            let mut pmi = PopupMenuItem::new(label).disabled(disabled);
            if let Some(icon) = icon {
                pmi = pmi.icon(icon);
            }
            if let Some(cmd) = command {
                pmi = pmi.on_click(move |_, _window, cx| cmd.execute(&(), cx));
            }
            menu = menu.item(pmi);
        }
    }
    menu
}

//! MVVM 菜单数据契约 + `items={...}` 运行时渲染
//!
//! - **声明式菜单**（`<menu-bar>` / `<context-menu>` / `<dropdown-menu>` + `<menu-item>`）
//!   由 engine `compiler/menu/` 直译 gpui-component `PopupMenu` API，不经过本模块。
//! - **数据绑定**（`<menu items={menu_items} />`）由 ViewModel 提供 `MenuItems`，
//!   本模块在运行时把 `IMenuItem` 树渲染为水平菜单栏。

use std::sync::Arc;

use gpui::{App, Context, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window};
use gpui_component::{
    IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    separator::Separator,
    Disableable as _,
};

/// 菜单项接口（object-safe）
pub trait IMenuItem: Send + Sync + 'static {
    fn label(&self) -> SharedString;
    fn icon(&self) -> Option<IconName> {
        None
    }
    fn disabled(&self) -> bool {
        false
    }
    fn separator(&self) -> bool {
        false
    }
    /// 分组标题（非可点击项）
    fn header(&self) -> bool {
        false
    }
    fn checked(&self) -> bool {
        false
    }
    fn href(&self) -> Option<SharedString> {
        None
    }
    fn command(&self) -> Option<Arc<dyn rml_core::command::ICommand>> {
        None
    }
    fn children(&self) -> Option<Vec<Arc<dyn IMenuItem>>> {
        None
    }
}

pub type MenuItems = Vec<Arc<dyn IMenuItem>>;

/// 菜单项默认实现
pub struct MenuItem {
    label: SharedString,
    icon: Option<IconName>,
    disabled: bool,
    separator: bool,
    header: bool,
    checked: bool,
    href: Option<SharedString>,
    command: Option<Arc<dyn rml_core::command::ICommand>>,
    children: Vec<Arc<dyn IMenuItem>>,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
            separator: false,
            header: false,
            checked: false,
            href: None,
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

    pub fn header(mut self) -> Self {
        self.header = true;
        self
    }

    pub fn checked(mut self, c: bool) -> Self {
        self.checked = c;
        self
    }

    pub fn href(mut self, url: impl Into<SharedString>) -> Self {
        self.href = Some(url.into());
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
    fn label(&self) -> SharedString {
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

    fn header(&self) -> bool {
        self.header
    }

    fn checked(&self) -> bool {
        self.checked
    }

    fn href(&self) -> Option<SharedString> {
        self.href.clone()
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

/// Menu 容器（兼容小写 `<menu items={...}>`）
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
        render_menu_bar_from_items(self.items)
    }
}

/// 从 `MenuItems` 渲染水平菜单栏（供 Menu 与 codegen 复用）
pub fn render_menu_bar_from_items(items: MenuItems) -> impl IntoElement {
    let mut bar = h_flex().h_full().items_center();

    for (ix, item) in items.iter().enumerate() {
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
            let mut btn = Button::new(("rml-menu", ix))
                .label(label)
                .ghost()
                .disabled(disabled);

            if let Some(icon) = icon {
                btn = btn.icon(icon);
            }

            let btn = btn.dropdown_menu(move |menu, window, cx| {
                build_popup_menu_from_items(menu, &children, window, cx)
            });
            bar = bar.child(btn);
        } else {
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

/// 从 `IMenuItem` 树递归构建 gpui-component `PopupMenu`（MVVM 子菜单路径专用）。
fn build_popup_menu_from_items(
    mut menu: PopupMenu,
    items: &MenuItems,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    for item in items {
        if item.separator() {
            menu = menu.separator();
            continue;
        }

        if item.header() {
            menu = menu.label(item.label());
            continue;
        }

        let label = item.label();
        let disabled = item.disabled();
        let icon = item.icon();
        let checked = item.checked();
        let href = item.href();
        let children = item.children();
        let command = item.command();

        if let Some(children) = children {
            let submenu_label = label;
            menu = if let Some(icon) = icon {
                menu.submenu_with_icon(Some(icon.into()), submenu_label, window, cx, {
                    let children = children.clone();
                    move |submenu, window, cx| {
                        build_popup_menu_from_items(submenu, &children, window, cx)
                    }
                })
            } else {
                menu.submenu(submenu_label, window, cx, {
                    let children = children.clone();
                    move |submenu, window, cx| {
                        build_popup_menu_from_items(submenu, &children, window, cx)
                    }
                })
            };
            continue;
        }

        if let Some(href) = href {
            if let Some(icon) = icon.clone() {
                menu = menu.link_with_icon(label, icon, href);
            } else {
                menu = menu.link(label, href);
            }
            continue;
        }

        let mut pmi = PopupMenuItem::new(label)
            .disabled(disabled)
            .checked(checked);
        if let Some(icon) = icon {
            pmi = pmi.icon(icon);
        }
        if let Some(cmd) = command {
            pmi = pmi.on_click(move |_, _window, cx| cmd.execute(&(), cx));
        }
        menu = menu.item(pmi);
    }
    menu
}

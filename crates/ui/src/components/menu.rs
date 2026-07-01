//! `MenuBar` —— RML 水平菜单栏（gpui-component 无此组件，由本 crate 定义）
//!
//! - **`<menu-bar>` / `<menu items={...}>`**：顶层入口由 `MenuBar` 渲染；声明式子节点
//!   由 engine `compiler/menu/menu_bar.rs` 编译为 `menu_bar_button` + `PopupMenu` 后作为
//!   `MenuBar` 的 children 传入。
//! - **`<context-menu>` / `<dropdown-menu>`**：弹层菜单容器，仍由 engine `compiler/menu/`
//!   直译 gpui-component `PopupMenu` API（非菜单栏）。
//! - **MVVM**：ViewModel 提供 `MenuItems`，`MenuBar::items(...)` 在运行时翻译为按钮树。

use std::sync::Arc;

use gpui::{
    px, AnyElement, App, Context, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window,
};
use gpui_component::{
    IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    separator::Separator,
    Disableable as _, StyledExt as _,
};
use smallvec::SmallVec;

/// 菜单栏顶层按钮默认外边距（px）
pub const MENU_BAR_BUTTON_MARGIN_PX: f32 = 2.;
/// 菜单栏顶层按钮默认左右内边距（px）
pub const MENU_BAR_BUTTON_PAD_X_PX: f32 = 4.;
/// 菜单栏下拉菜单默认最小宽度（px）
pub const MENU_BAR_POPUP_MIN_W_PX: f32 = 250.;

/// 声明式 codegen 与 MVVM 共用的菜单栏顶层按钮样式
pub fn menu_bar_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button::new(id)
        .label(label)
        .ghost()
        .margins(px(MENU_BAR_BUTTON_MARGIN_PX))
        .pl(px(MENU_BAR_BUTTON_PAD_X_PX))
        .pr(px(MENU_BAR_BUTTON_PAD_X_PX))
}

/// 菜单栏下拉弹层默认配置（声明式 codegen 与 MVVM 共用）
pub fn configure_menu_bar_popup(menu: PopupMenu) -> PopupMenu {
    menu.min_w(px(MENU_BAR_POPUP_MIN_W_PX))
}

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

/// `<menu items={...}>` 兼容别名
pub type Menu = MenuBar;

/// 水平菜单栏（声明式 children 或 MVVM `items` 二选一）
#[derive(IntoElement)]
pub struct MenuBar {
    id: ElementId,
    items: MenuItems,
    entry_children: SmallVec<[AnyElement; 4]>,
}

impl MenuBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            entry_children: SmallVec::new(),
        }
    }

    pub fn items(mut self, items: MenuItems) -> Self {
        self.items = items;
        self
    }
}

impl ParentElement for MenuBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.entry_children.extend(elements);
    }
}

impl RenderOnce for MenuBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut bar = h_flex()
            .id(self.id)
            .h_full()
            .items_center();

        if !self.items.is_empty() {
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
                    let mut btn = menu_bar_button(("rml-menu", ix), label).disabled(disabled);

                    if let Some(icon) = icon {
                        btn = btn.icon(icon);
                    }

                    let btn = btn.dropdown_menu(move |menu, window, cx| {
                        build_popup_menu_from_items(
                            configure_menu_bar_popup(menu),
                            &children,
                            window,
                            cx,
                        )
                    });
                    bar = bar.child(btn);
                } else {
                    let mut btn = menu_bar_button(("rml-menu", ix), label).disabled(disabled);

                    if let Some(icon) = icon {
                        btn = btn.icon(icon);
                    }

                    if let Some(cmd) = command {
                        btn = btn.on_click(move |_, _window, cx| cmd.execute(&(), cx));
                    }
                    bar = bar.child(btn);
                }
            }
        } else {
            bar = bar.children(self.entry_children);
        }

        bar
    }
}

/// 从 `MenuItems` 渲染水平菜单栏（MVVM 与 `MenuBar::items` 共用）
pub fn render_menu_bar_from_items(items: MenuItems) -> impl IntoElement {
    MenuBar::new("rml-menu-bar").items(items)
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

//! Menu —— MVVM 数据绑定的菜单栏组件
//!
//! 参照 `activity_bar.rs` 黄金模板：`IMenuItem` trait + `MenuItem` 默认实现 +
//! `MenuItems = Vec<Arc<dyn IMenuItem>>` 类型别名 + `Menu` 容器组件。
//!
//! ViewModel 通过 `#[computed]` 返回 `MenuItems`，在 RML 中 `<menu items={menu_items} />` 绑定。
//! 每个菜单项可携带 `Arc<dyn ICommand>` 命令对象，点击时调用 `execute`。

use std::sync::Arc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, prelude::FluentBuilder as _};
use gpui_component::{
    IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    separator::Separator,
    Disableable as _,
};

/// 菜单项接口（object-safe，可存储为 `Arc<dyn IMenuItem>`）
pub trait IMenuItem: 'static {
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
}

impl MenuItem {
    pub fn new(label: impl Into<gpui::SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
            separator: false,
            command: None,
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
}

/// Menu 容器组件
///
/// Stateless 构造：`Menu::new(id).items(items)`
/// 通过 `items()` 接收 `MenuItems` 数据绑定。
#[derive(IntoElement)]
pub struct Menu {
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
        let _id = self.id;
        let mut bar = h_flex().h_full().items_center();

        for (ix, item) in self.items.iter().enumerate() {
            if item.separator() {
                bar = bar.child(Separator::vertical().h_full());
                continue;
            }

            let cmd = item.command();
            let disabled = item.disabled();
            let label = item.label();
            let icon = item.icon();

            let mut btn = Button::new(("menu-item", ix))
                .label(label)
                .ghost()
                .disabled(disabled);

            if let Some(icon) = icon {
                btn = btn.icon(icon);
            }

            if let Some(cmd) = cmd {
                btn = btn.on_click(move |_, _window, cx| cmd.execute(&(), cx));
            }

            bar = bar.child(btn);
        }

        bar
    }
}

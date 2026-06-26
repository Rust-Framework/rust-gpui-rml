//! 窗口数据类型 —— 用于 MVVM 绑定的纯数据结构
//!
//! `MenuItem` / `StatusBarItem` 是 ViewModel 持有的数据字段，
//! 在 `.rml` 中通过 `menu={self.menu_items}` / `status_bar={self.status_items}` 绑定到 `<ModernWindowShell>`。
//!
//! 闭包捕获 `WeakEntity<T>`，符合 GPUI 事件模式，避免引入字符串命令派发的间接层。

use std::rc::Rc;

use gpui::{App, SharedString, Window};

/// 菜单项数据（用于 MVVM 绑定）
#[derive(Clone)]
pub struct MenuItem {
    pub label: SharedString,
    pub on_click: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    pub disabled: bool,
    pub checked: bool,
    pub children: Vec<MenuItem>,
    pub separator: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            disabled: false,
            checked: false,
            children: Vec::new(),
            separator: false,
        }
    }

    pub fn separator() -> Self {
        let mut item = Self::new("");
        item.separator = true;
        item
    }

    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }

    /// 类似 `cx.listener` 的闭包绑定
    ///
    /// 用法：`MenuItem::new("Save").on_click_with(cx, |this, window, cx| this.save(window, cx))`
    pub fn on_click_with<T, F>(self, cx: &gpui::Context<T>, f: F) -> Self
    where
        T: 'static,
        F: Fn(&mut T, &mut Window, &mut gpui::App) + 'static,
    {
        let weak = cx.weak_entity();
        self.on_click(move |window, cx| {
            if let Some(this) = weak.upgrade() {
                this.update(cx, |this, cx| f(this, window, cx));
            }
        })
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn submenu(mut self, children: Vec<MenuItem>) -> Self {
        self.children = children;
        self
    }
}

/// 状态栏项数据
#[derive(Clone)]
pub struct StatusBarItem {
    pub label: SharedString,
    pub on_click: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    pub icon: Option<SharedString>,
}

impl StatusBarItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            icon: None,
        }
    }

    pub fn on_click_with<T, F>(self, cx: &gpui::Context<T>, f: F) -> Self
    where
        T: 'static,
        F: Fn(&mut T, &mut Window, &mut gpui::App) + 'static,
    {
        let weak = cx.weak_entity();
        StatusBarItem {
            on_click: Some(Rc::new(move |window, cx| {
                if let Some(this) = weak.upgrade() {
                    this.update(cx, |this, cx| f(this, window, cx));
                }
            })),
            ..self
        }
    }
}

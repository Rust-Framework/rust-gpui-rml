//! RML 扩展组件库
//!
//! 基于 `gpui-component` 的 **re-export 轨** + **窗口壳** + **MVVM 绑定适配**。
//! 声明式菜单（`ContextMenu`/`DropdownMenu`/`MenuBar` + `MenuItem`）由 engine
//! `compiler/menu/` 直译 gpui-component API，不在此 crate 重复包装。
//!
//! ## 使用方式
//!
//! 通常无需直接调用本 crate —— `rml-app` 在 feature `ui-components` 启用时会自动调用 [`init`]。
//! 如需在用户代码中直接使用组件：
//!
//! ```rust,ignore
//! use rml_ui::prelude::*;
//!
//! // 在 render 方法中
//! gpui::div().child(
//!     rml_ui::Button::new("my-btn")
//!         .label("Click me")
//!         .on_click(|_, _, _| println!("clicked"))
//! )
//! ```

#![forbid(unsafe_code)]

// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_core as rml_core;

pub mod prelude;
pub mod components;
pub mod window;

/// 初始化扩展组件。
///
/// 必须在窗口创建前调用，通常由 `RmlApplication::run` 在 feature 启用时自动调用。
/// 内部依次初始化：theme / global_state / root / focus_trap / dialog / sheet / list 等模块。
pub fn init(cx: &mut gpui::App) {
    gpui_component::init(cx);
    rml_core::i18n::ensure_i18n(cx);
}

// 直接 re-export 高频组件，避免在每个使用点写完整路径
pub use gpui_component::{
    Icon, IconName, Root, TitleBar, WindowExt,
    accordion::{Accordion, AccordionItem},
    badge::Badge,
    button::{Button, ButtonGroup},
    checkbox::Checkbox,
    dialog::Dialog,
    form::Form,
    input::{Input, InputEvent, InputState},
    kbd::Kbd,
    label::Label,
    list::List,
    menu::{AppMenuBar, ContextMenuExt, DropdownMenu, PopupMenuItem},
    notification::{Notification, NotificationList, NotificationType},
    popover::Popover,
    progress::{Progress, ProgressCircle},
    radio::Radio,
    select::Select,
    separator::Separator,
    slider::Slider,
    switch::Switch,
    table::Table,
    tag::Tag,
    tooltip::Tooltip,
    tree::{TreeEntry, TreeEvent, TreeItem, TreeState},
    Side, h_flex, v_flex,
};

// 共享 trait 体系
pub use gpui_component::{
    button::ButtonVariants, Disableable, Sizable, Selectable, StyledExt,
};

// 窗口组件：ModernWindowShell 内置封装 + 内置 Window/ModernWindow + 助手 trait
pub use window::{
    IWindowActions, IWindowExt, ModernWindow, ModernWindowShell, NotificationKind, TabItem,
    TabWindowShell, Window,
};

pub use components::{
    ActivityAct, ActivityActs, ActivityBar, ActivityPanel, ActivityPanels, DialogDragState,
    DialogTitleBar, IActivityAct, IActivityPanel, IMenuItem, IStatusBarItem, Menu, MenuBar,
    MenuItem, MenuItems, NativeStatusBar, StatusBar, StatusBarAlign, StatusBarItem,
    StatusBarItems, Tab, TabBar, TabVariant, Tree, VisualActivityPanel, configure_menu_bar_popup,
    menu_bar_button, render_menu_bar_from_items,
};

//! RML 扩展组件库
//!
//! 基于 `gpui-component` 提供的高级组件封装（Button / Input / Dialog / List / Form 等）。
//! 通过 feature `ui-components`（默认开启）启用，关闭后退化为空实现。
//!
//! ## 设计目标
//!
//! - **零成本抽象**：直接 re-export `gpui-component` 类型，避免不必要的 wrapper struct
//! - **RML 集成入口**：通过 [`init`] 完成 gpui-component 的全局初始化
//! - **codegen 路由目标**：PascalCase 标签（`<Button>`/`<Input>`）在 codegen 时映射到本 crate 的构造器
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
    badge::Badge,
    button::{Button, ButtonGroup},
    checkbox::Checkbox,
    dialog::Dialog,
    form::Form,
    input::{Input, InputEvent, InputState},
    kbd::Kbd,
    label::Label,
    list::List,
    notification::{Notification, NotificationList, NotificationType},
    popover::Popover,
    progress::{Progress, ProgressCircle},
    radio::Radio,
    select::Select,
    separator::Separator,
    slider::Slider,
    status_bar::StatusBar,
    switch::Switch,
    tab::{Tab, TabBar},
    table::Table,
    tag::Tag,
    tooltip::Tooltip,
    tree::{TreeEntry, TreeEvent, TreeItem, TreeState},
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
    ActivityAct, ActivityActs, ActivityBar, ActivityPanel, ActivityPanels,
    IActivityAct, IActivityPanel, TreeView,
};

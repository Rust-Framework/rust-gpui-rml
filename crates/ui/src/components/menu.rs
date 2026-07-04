//! `MenuBar` —— RML 水平菜单栏（gpui-component 无此组件，由本 crate 定义）
//!
//! - **`<menu-bar>` / `<menu items={...}>`**：顶层入口由 `MenuBar` 渲染；声明式子节点
//!   由 engine `compiler/menu/menu_bar.rs` 编译为 `menu_bar_button` + `PopupMenu` 后作为
//!   `MenuBar` 的 children 传入。
//! - **`<context-menu>` / `<dropdown-menu>`**：弹层菜单容器，仍由 engine `compiler/menu/`
//!   直译 gpui-component `PopupMenu` API（非菜单栏）。
//! - **MVVM**：业务定义自己的 `MenuViewModel`，`MainWindow::render_menu_bar()` 构建
//!   `menu_bar_button` + `dropdown_menu` 闭包后作为 `MenuBar` 的 children 传入。

use gpui::{
    px, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::PopupMenu,
};
use smallvec::SmallVec;

/// 菜单栏顶层按钮默认上下外边距（px）
pub const MENU_BAR_BUTTON_MARGIN_PX: f32 = 2.;
/// 菜单栏顶层按钮默认左右内边距（px）
pub const MENU_BAR_BUTTON_PAD_X_PX: f32 = 6.;
/// 菜单栏顶层按钮默认上下内边距（px）
pub const MENU_BAR_BUTTON_PAD_Y_PX: f32 = 2.;
/// 菜单栏按钮间距（px）
pub const MENU_BAR_GAP_PX: f32 = 4.;
/// 菜单栏下拉菜单默认最小宽度（px）
pub const MENU_BAR_POPUP_MIN_W_PX: f32 = 250.;

/// 声明式 codegen 与 MVVM 共用的菜单栏顶层按钮样式
pub fn menu_bar_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    styled_menu_bar_button(
        id,
        label,
        MENU_BAR_BUTTON_MARGIN_PX,
        MENU_BAR_BUTTON_PAD_X_PX,
        MENU_BAR_BUTTON_PAD_Y_PX,
    )
}

fn styled_menu_bar_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    margin_y: f32,
    pad_x: f32,
    pad_y: f32,
) -> Button {
    Button::new(id)
        .label(label)
        .ghost()
        .h(px(22.))
        .my(px(margin_y))
        .px(px(pad_x))
        .py(px(pad_y))
}

/// 菜单栏下拉弹层默认配置（声明式 codegen 与 MVVM 共用）
pub fn configure_menu_bar_popup(menu: PopupMenu) -> PopupMenu {
    menu.min_w(px(MENU_BAR_POPUP_MIN_W_PX))
}

/// `<menu items={...}>` 兼容别名
pub type Menu = MenuBar;

/// 水平菜单栏（纯 `ParentElement` 容器——接收 `.child(...)` / `.children(...)`）
///
/// 框架不定义 `IMenuItem` 数据结构（WPF 风格——业务定义自己的 ViewModel）。
/// 业务侧构建 `menu_bar_button` + `dropdown_menu` 闭包后经 `.child(...)` 传入。
#[derive(IntoElement)]
pub struct MenuBar {
    id: ElementId,
    entry_children: SmallVec<[gpui::AnyElement; 4]>,
    gap: f32,
    button_margin_y: f32,
    button_pad_x: f32,
    button_pad_y: f32,
}

impl MenuBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entry_children: SmallVec::new(),
            gap: MENU_BAR_GAP_PX,
            button_margin_y: MENU_BAR_BUTTON_MARGIN_PX,
            button_pad_x: MENU_BAR_BUTTON_PAD_X_PX,
            button_pad_y: MENU_BAR_BUTTON_PAD_Y_PX,
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn button_margin(mut self, margin_y: f32) -> Self {
        self.button_margin_y = margin_y;
        self
    }

    pub fn button_pad_x(mut self, pad_x: f32) -> Self {
        self.button_pad_x = pad_x;
        self
    }

    pub fn button_pad_y(mut self, pad_y: f32) -> Self {
        self.button_pad_y = pad_y;
        self
    }
}

impl ParentElement for MenuBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.entry_children.extend(elements);
    }
}

impl RenderOnce for MenuBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        h_flex()
            .id(self.id)
            .h_full()
            .items_center()
            .gap(px(self.gap))
            .children(self.entry_children)
    }
}

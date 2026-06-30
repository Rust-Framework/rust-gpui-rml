//! 窗口模板组件 —— Menu / StatusBar 插槽的默认渲染模板

use gpui::{IntoElement, RenderOnce, Styled, Window, div};

use super::menu_bar::render_menu_bar;
use super::modern_window::render_status_bar;
use super::types::{MenuItem, StatusBarItem};

/// 菜单栏模板（包装 `render_menu_bar`）
#[derive(IntoElement)]
pub struct MenuBarTemplate {
    items: Vec<MenuItem>,
}

impl MenuBarTemplate {
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self { items }
    }
}

impl RenderOnce for MenuBarTemplate {
    fn render(self, _window: &mut Window, _cx: &mut gpui::App) -> impl IntoElement {
        render_menu_bar(&self.items)
    }
}

/// 状态栏模板（包装 status bar 渲染）
#[derive(IntoElement)]
pub struct StatusBarTemplate {
    items: Vec<StatusBarItem>,
}

impl StatusBarTemplate {
    pub fn new(items: Vec<StatusBarItem>) -> Self {
        Self { items }
    }
}

impl RenderOnce for StatusBarTemplate {
    fn render(self, _window: &mut Window, _cx: &mut gpui::App) -> impl IntoElement {
        render_status_bar(&self.items)
    }
}

/// 空插槽占位（隐藏）
pub fn empty_slot() -> impl IntoElement {
    div().hidden()
}

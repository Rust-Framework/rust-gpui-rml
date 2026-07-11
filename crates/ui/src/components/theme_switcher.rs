//! ThemeSwitcher —— 声明式主题切换器
//!
//! RML `<ThemeSwitcher value={current_theme} />` 在 render 时自动调用
//! `cx.set_theme(value)` 切换全局主题（light/dark）。
//!
//! 组件不渲染可见内容（返回空 div），仅作为声明式副作用触发器。
//! `set_theme` 内部仅在主题实际变化时刷新窗口，因此每次 render 调用是安全的。

use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div};
use rml_core::theme::ThemeExt;

/// 声明式主题切换器
///
/// 绑定 `value` 属性到 ViewModel 的 `SharedString` / `String` 字段，
/// render 时自动调用 `cx.set_theme()`。当值与当前主题相同时为空操作。
#[derive(IntoElement)]
pub struct ThemeSwitcher {
    value: SharedString,
}

impl Default for ThemeSwitcher {
    fn default() -> Self {
        Self {
            value: SharedString::default(),
        }
    }
}

impl ThemeSwitcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }
}

impl RenderOnce for ThemeSwitcher {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.value.is_empty() && cx.current_theme().as_ref() != self.value.as_ref() {
            cx.set_theme(self.value);
        }
        div()
    }
}

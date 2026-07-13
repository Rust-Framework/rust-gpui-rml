//! 消息内容渲染模式。
//!
//! 控制消息气泡中的内容如何渲染：
//! - [`PlainText`]: 纯文本（IM 默认）
//! - [`Markdown`]: Markdown 富文本（AI 默认），使用 RML 的 [`Markdown`](crate::Markdown) 组件

use crate::Markdown as MarkdownView;
use gpui::{AnyElement, App, IntoElement, ParentElement, SharedString, Window};

/// 消息渲染模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// 纯文本渲染（IM 默认）
    #[default]
    PlainText,
    /// Markdown 富文本渲染（AI 默认）
    Markdown,
}

/// 将消息内容渲染为元素。
pub fn render_content(
    mode: RenderMode,
    content: &str,
    _window: &mut Window,
    _cx: &mut App,
) -> AnyElement {
    match mode {
        RenderMode::PlainText => gpui::div()
            .child(SharedString::from(content.to_string()))
            .into_any_element(),
        RenderMode::Markdown => MarkdownView::new()
            .content(SharedString::from(content.to_string()))
            .into_any_element(),
    }
}

//! Markdown 富文本渲染组件
//!
//! 基于 `gpui-component` 的 `TextView::markdown`，支持 GFM Markdown 语法：
/// 标题、段落、粗体/斜体/删除线、行内代码、代码块（语法高亮）、
/// 链接、图片、引用块、列表、表格、水平线等。
///
/// 声明式语法：`<Markdown content={text} />`
///
/// 适用于 AI 聊天输出、帮助文档、发布说明等富文本展示场景。
use std::sync::atomic::{AtomicUsize, Ordering};

use gpui::{
    App, ElementId, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
};
use gpui_component::text::TextView;
use gpui_component::StyledExt as _;

static MARKDOWN_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Markdown 富文本渲染组件
///
/// 通过 `content` 属性传入 Markdown 文本，自动解析并渲染为富文本。
#[derive(IntoElement)]
pub struct Markdown {
    content: SharedString,
    style: StyleRefinement,
}

impl Default for Markdown {
    fn default() -> Self {
        Self {
            content: SharedString::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl Markdown {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(mut self, content: impl Into<SharedString>) -> Self {
        self.content = content.into();
        self
    }
}

impl Styled for Markdown {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Markdown {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let id = MARKDOWN_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        TextView::markdown(
            ElementId::Name(format!("rml-markdown-{}", id).into()),
            self.content,
        )
        .refine_style(&self.style)
    }
}

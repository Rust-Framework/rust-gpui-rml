//! 单条消息气泡组件。

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::ActiveTheme;

use super::model::{Message, MessageRole};
use super::renderer::{render_content, RenderMode};

/// 消息气泡。
///
/// 根据 [`MessageRole`] 选择左/右对齐和颜色样式。
#[derive(IntoElement)]
pub struct ChatBubble {
    message: Message,
    render_mode: RenderMode,
}

impl ChatBubble {
    pub fn new(message: Message, render_mode: RenderMode) -> Self {
        Self {
            message,
            render_mode,
        }
    }
}

impl RenderOnce for ChatBubble {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let is_user = self.message.role.is_user();
        let is_system = matches!(self.message.role, MessageRole::System);

        let bubble_bg = if is_user {
            theme.accent
        } else if is_system {
            theme.muted
        } else {
            theme.muted
        };
        let text_color = if is_user {
            theme.accent_foreground
        } else {
            theme.foreground
        };

        let content = render_content(self.render_mode, &self.message.content, window, cx);

        div()
            .flex()
            .w_full()
            .when(is_user, |d| d.justify_end())
            .when(!is_user, |d| d.justify_start())
            .child(
                div()
                    .max_w(px(720.))
                    .px_3()
                    .py_2()
                    .mb_2()
                    .rounded_lg()
                    .bg(bubble_bg)
                    .text_color(text_color)
                    .child(content),
            )
    }
}

//! 消息列表组件。
//!
//! 将 `Vec<Message>` 渲染为可滚动的消息气泡列表。

use gpui::*;
use gpui_component::ActiveTheme;

use crate::message_bubble::ChatBubble;
use crate::model::Message;
use crate::renderer::RenderMode;

/// 消息列表事件。
#[derive(Debug, Clone)]
pub enum MessageListEvent {
    ScrollToBottom,
}

/// 消息列表视图。
pub struct MessageListView {
    messages: Vec<Message>,
    render_mode: RenderMode,
}

impl MessageListView {
    pub fn new(render_mode: RenderMode) -> Self {
        Self {
            messages: Vec::new(),
            render_mode,
        }
    }

    pub fn set_messages(&mut self, messages: Vec<Message>, cx: &mut Context<Self>) {
        self.messages = messages;
        cx.notify();
    }

    pub fn add_message(&mut self, message: Message, cx: &mut Context<Self>) {
        self.messages.push(message);
        cx.notify();
    }

    pub fn update_last_message(&mut self, content: &str, cx: &mut Context<Self>) {
        if let Some(last) = self.messages.last_mut() {
            last.content = content.to_string();
            cx.notify();
        }
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
}

impl EventEmitter<MessageListEvent> for MessageListView {}

impl Render for MessageListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let messages: Vec<Message> = self.messages.clone();
        let mode = self.render_mode;

        div()
            .id("chat-message-list")
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .bg(theme.background)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p_4()
                    .gap_1()
                    .children(
                        messages
                            .into_iter()
                            .map(move |msg| ChatBubble::new(msg, mode)),
                    ),
            )
    }
}

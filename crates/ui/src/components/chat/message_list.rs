//! 消息列表组件。
//!
//! 视觉规范对齐源项目 `agent/src/chat/message_list_view.rs`：
//! - 空状态：56x56 Bot 图标 + 欢迎标题 + 提示文案
//! - 列表：px=16, py=8, overflow_y_scroll
//! - 消息间距：mb=12（由 ChatBubble 处理）

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable as _};

use super::action::{ChatMessageAction, MessageActionItem};

use super::message_bubble::ChatBubble;
use super::model::ChatMessage;
use super::renderer::RenderMode;

/// 消息列表事件。
#[derive(Debug, Clone)]
pub enum MessageListEvent {
    ScrollToBottom,
    /// 用户对某条消息执行操作。
    MessageAction {
        message_id: u64,
        action: ChatMessageAction,
    },
}

/// 消息列表视图。
pub struct MessageListView {
    messages: Vec<ChatMessage>,
    render_mode: RenderMode,
    custom_actions: Vec<MessageActionItem>,
}

impl MessageListView {
    pub fn new(render_mode: RenderMode) -> Self {
        Self {
            messages: Vec::new(),
            render_mode,
            custom_actions: Vec::new(),
        }
    }

    pub fn set_custom_actions(&mut self, actions: Vec<MessageActionItem>, cx: &mut Context<Self>) {
        self.custom_actions = actions;
        cx.notify();
    }

    pub fn custom_actions(&self) -> &[MessageActionItem] {
        &self.custom_actions
    }

    pub fn set_messages(&mut self, messages: Vec<ChatMessage>, cx: &mut Context<Self>) {
        self.messages = messages;
        cx.notify();
    }

    pub fn add_message(&mut self, message: ChatMessage, cx: &mut Context<Self>) {
        self.messages.push(message);
        cx.notify();
    }

    pub fn update_last_message(&mut self, content: &str, cx: &mut Context<Self>) {
        if let Some(last) = self.messages.last_mut() {
            last.content = content.to_string();
            cx.notify();
        }
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// 渲染空状态。
    fn render_empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(16.))
            .child(
                div()
                    .w(px(56.))
                    .h(px(56.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(theme.accent.opacity(0.1))
                    .child(
                        Icon::new(IconName::Bot)
                            .with_size(px(28.))
                            .text_color(theme.accent),
                    ),
            )
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child("开始对话"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .w(px(320.))
                    .text_align(TextAlign::Center)
                    .child("输入消息开始与 AI 助手对话，支持 Markdown 格式渲染"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(
                        Icon::new(IconName::Info)
                            .with_size(px(12.))
                            .text_color(theme.muted_foreground),
                    )
                    .child("按 Enter 发送，Shift+Enter 换行"),
            )
    }
}

impl EventEmitter<MessageListEvent> for MessageListView {}

impl Render for MessageListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let has_messages = !self.messages.is_empty();
        let messages: Vec<ChatMessage> = self.messages.clone();
        let mode = self.render_mode;
        let custom_actions = self.custom_actions.clone();

        div()
            .id("chat-message-list-host")
            .h_full()
            .w_full()
            .min_w_0()
            .min_h_0()
            .relative()
            .bg(theme.background)
            .when(!has_messages, |el| el.child(self.render_empty_state(cx)))
            .when(has_messages, |el| {
                el.child(
                    div()
                        .id("chat-message-list")
                        .h_full()
                        .w_full()
                        .min_w_0()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .px(px(16.))
                        .py(px(16.))
                        .children(
                            messages
                                .into_iter()
                                .map(|msg| ChatBubble::new(msg, mode, &cx.entity(), custom_actions.clone())),
                        ),
                )
            })
    }
}

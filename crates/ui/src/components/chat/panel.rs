//! 聊天面板主组件。
//!
//! [`ChatPanel`] 是一个 GPUI View，组合 [`MessageListView`]（消息列表）与 [`ChatInput`]（输入区），
//! 通过 [`ChatBackend`] trait 对接不同的聊天后端（IM 同步响应 / AI 流式响应）。

use std::sync::{Arc, Mutex};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::ActiveTheme;

use super::backend::{ChatBackend, ChatError};
use super::event::{ChatEvent, ChatInputEvent};
use super::input::ChatInput;
use super::message_list::MessageListView;
use super::model::{Conversation, Message};
use super::renderer::RenderMode;

/// 通用聊天面板。
///
/// 通过 [`ChatPanel::new`] 创建空面板，再通过 [`ChatPanel::set_backend`] 注入后端实现。
/// 用户在 ViewModel 的 `on_loaded` 中完成初始化。
pub struct ChatPanel {
    conversation: Conversation,
    backend: Option<Arc<dyn ChatBackend>>,
    render_mode: RenderMode,
    input: Option<Entity<ChatInput>>,
    _input_sub: Option<Subscription>,
    message_list: Option<Entity<MessageListView>>,
    _message_list_sub: Option<Subscription>,
    focus_handle: FocusHandle,
    next_message_id: u64,
    is_streaming: bool,
}

impl ChatPanel {
    /// 创建空聊天面板，后续通过 `set_backend` 注入后端。
    pub fn new(render_mode: RenderMode, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let message_list = cx.new(|_| MessageListView::new(render_mode));
        let input = cx.new(|cx| ChatInput::new("输入消息...", window, cx));

        let _input_sub = cx.subscribe_in(
            &input,
            window,
            |this: &mut ChatPanel, _input: &Entity<ChatInput>, event: &ChatInputEvent, _window, cx| {
                match event {
                    ChatInputEvent::Send(text) => this.send_message(text.clone(), cx),
                    ChatInputEvent::Stop => this.cancel(cx),
                }
            },
        );

        Self {
            conversation: Conversation::new(0, "New Conversation"),
            backend: None,
            render_mode,
            input: Some(input),
            _input_sub: Some(_input_sub),
            message_list: Some(message_list),
            _message_list_sub: None,
            focus_handle,
            next_message_id: 0,
            is_streaming: false,
        }
    }

    /// 注入聊天后端。后端实现 `ChatBackend` trait，决定消息如何处理。
    pub fn set_backend(&mut self, backend: Arc<dyn ChatBackend>, cx: &mut Context<Self>) {
        self.backend = Some(backend);
        cx.notify();
    }

    /// 发送一条用户消息，并触发后端响应。
    pub fn send_message(&mut self, content: String, cx: &mut Context<Self>) {
        let Some(backend) = self.backend.clone() else {
            cx.emit(ChatEvent::Error("backend not configured".into()));
            return;
        };
        if self.is_streaming {
            return;
        }

        let user_msg = Message::user(self.next_message_id, &content);
        self.next_message_id += 1;
        self.conversation.add_message(user_msg.clone());
        if let Some(list) = &self.message_list {
            list.update(cx, |list, cx| list.add_message(user_msg, cx));
        }

        let assistant_id = self.next_message_id;
        self.next_message_id += 1;
        let assistant_msg = Message::assistant(assistant_id, "").with_streaming(true);
        self.conversation.add_message(assistant_msg);
        if let Some(list) = &self.message_list {
            list.update(cx, |list, cx| {
                list.add_message(Message::assistant(assistant_id, "").with_streaming(true), cx)
            });
        }

        self.is_streaming = true;
        if let Some(input) = &self.input {
            input.update(cx, |input, cx| input.set_streaming(true, cx));
        }

        let conv = self.conversation.clone();
        cx.spawn(async move |this, cx| {
            let accumulated: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
            let acc_clone = accumulated.clone();

            let result = cx
                .background_executor()
                .spawn(async move {
                    backend.stream_message(&conv, &content, &move |chunk: &str| {
                        if let Ok(mut guard) = acc_clone.lock() {
                            guard.push_str(chunk);
                        }
                    })
                })
                .await;

            let full_content = accumulated.lock().map(|g| g.clone()).unwrap_or_default();

            let _ = this.update(cx, |this, cx| {
                if let Some(list) = &this.message_list {
                    list.update(cx, |list, cx| list.update_last_message(&full_content, cx));
                }
                this.is_streaming = false;
                if let Some(input) = &this.input {
                    input.update(cx, |input, cx| input.set_streaming(false, cx));
                }
                match result {
                    Ok(()) => cx.emit(ChatEvent::MessageReceived(full_content)),
                    Err(ChatError::Cancelled) => cx.emit(ChatEvent::Cancelled),
                    Err(e) => cx.emit(ChatEvent::Error(e.to_string())),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 取消当前流式请求。
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        if !self.is_streaming {
            return;
        }
        if let Some(backend) = &self.backend {
            if let Err(e) = backend.cancel() {
                cx.emit(ChatEvent::Error(e.to_string()));
            }
        }
        self.is_streaming = false;
        if let Some(input) = &self.input {
            input.update(cx, |input, cx| input.set_streaming(false, cx));
        }
        cx.emit(ChatEvent::Cancelled);
        cx.notify();
    }

    /// 当前会话的全部消息。
    pub fn messages(&self) -> &[Message] {
        &self.conversation.messages
    }

    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }
}

impl EventEmitter<ChatEvent> for ChatPanel {}

impl Focusable for ChatPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ChatPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let input = self.input.clone();
        let message_list = self.message_list.clone();

        div()
            .id("chat-panel")
            .flex()
            .flex_col()
            .h_full()
            .w_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .track_focus(&self.focus_handle)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .when_some(message_list, |el, list| el.child(list)),
            )
            .when_some(input, |el, input| el.child(input))
    }
}

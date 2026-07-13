//! 聊天面板主组件。
//!
//! [`ChatPanel`] 是一个 GPUI View，组合头部栏 + [`MessageListView`]（消息列表）+ [`ChatInput`]（输入区），
//! 通过 [`IChatBackend`] trait 对接不同的聊天后端（IM 同步响应 / AI 流式响应）。
//!
//! 视觉规范对齐源项目 `default_chat_window.rs`：
//! - 44px 头部栏（icon + title，border-b，bg=background）
//! - 消息列表（flex_1，滚动）
//! - 输入区（border-t，bg=background）

use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{ActiveTheme, Icon, IconName};

use super::action::{ChatMessageAction, MessageActionItem};
use super::backend::{ChatError, ChatStreamEvent, IChatBackend};
use super::event::{ChatEvent, ChatInputEvent};
use super::input::ChatInput;
use super::message_list::{MessageListEvent, MessageListView};
use super::model::{
    ChatAttachment, ChatConfig, ChatConversation, ChatMessage, ChatRequest, ChatToolCall,
};
use super::renderer::RenderMode;

/// 流式响应累积状态。
struct StreamState {
    content: String,
    thinking: String,
    tool_calls: Vec<ChatToolCall>,
    attachments: Vec<ChatAttachment>,
}

/// 通用聊天面板。
///
/// 通过 [`ChatPanel::new`] 创建空面板，再通过 [`ChatPanel::set_backend`] 注入后端实现。
/// 用户在 ViewModel 的 `on_loaded` 中完成初始化。
pub struct ChatPanel {
    conversation: ChatConversation,
    backend: Option<Arc<dyn IChatBackend>>,
    render_mode: RenderMode,
    title: String,
    input: Option<Entity<ChatInput>>,
    _input_sub: Option<Subscription>,
    message_list: Option<Entity<MessageListView>>,
    _message_list_sub: Option<Subscription>,
    focus_handle: FocusHandle,
    next_message_id: u64,
    is_streaming: bool,
    message_actions: Vec<MessageActionItem>,
    on_message_action: Option<Rc<dyn Fn(&mut ChatPanel, u64, &ChatMessageAction, &mut Window, &mut Context<ChatPanel>)>>,
}

impl ChatPanel {
    /// 创建空聊天面板，后续通过 `set_backend` 注入后端。
    pub fn new(render_mode: RenderMode, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let message_list = cx.new(|_| MessageListView::new(render_mode));
        let input = cx.new(|cx| ChatInput::new("输入消息...", window, cx));

        let _message_list_sub = cx.subscribe_in(
            &message_list,
            window,
            |this: &mut ChatPanel,
             _list: &Entity<MessageListView>,
             event: &MessageListEvent,
             window,
             cx| {
                match event {
                    MessageListEvent::ScrollToBottom => {}
                    MessageListEvent::MessageAction { message_id, action } => {
                        let action = action.clone();
                        cx.emit(ChatEvent::MessageAction {
                            message_id: *message_id,
                            action: action.clone(),
                        });
                        if let Some(handler) = this.on_message_action.clone() {
                            handler(this, *message_id, &action, window, cx);
                        }
                    }
                }
            },
        );

        let _input_sub = cx.subscribe_in(
            &input,
            window,
            |this: &mut ChatPanel,
             _input: &Entity<ChatInput>,
             event: &ChatInputEvent,
             _window,
             cx| {
                match event {
                    ChatInputEvent::Send(text) => this.send_message(text.clone(), cx),
                    ChatInputEvent::Stop => this.cancel(cx),
                    ChatInputEvent::SelectModel(model_id) => {
                        this.conversation.config.model = Some(model_id.clone());
                        cx.emit(ChatEvent::ModelSelected(model_id.clone()));
                        cx.notify();
                    }
                }
            },
        );

        Self {
            conversation: ChatConversation::new(0, "New Conversation"),
            backend: None,
            render_mode,
            title: "Chat".to_string(),
            input: Some(input),
            _input_sub: Some(_input_sub),
            message_list: Some(message_list),
            _message_list_sub: Some(_message_list_sub),
            focus_handle,
            next_message_id: 0,
            is_streaming: false,
            message_actions: Vec::new(),
            on_message_action: None,
        }
    }

    /// 注入聊天后端。后端实现 `IChatBackend` trait，决定消息如何处理。
    pub fn set_backend(&mut self, backend: Arc<dyn IChatBackend>, cx: &mut Context<Self>) {
        self.backend = Some(backend);
        cx.notify();
    }

    /// 设置会话级配置（模型、温度等）。
    pub fn set_config(&mut self, config: ChatConfig, cx: &mut Context<Self>) {
        self.conversation.config = config;
        cx.notify();
    }

    /// 设置头部栏标题。
    pub fn set_title(&mut self, title: impl Into<String>, cx: &mut Context<Self>) {
        self.title = title.into();
        cx.notify();
    }

    /// 设置可用模型列表与当前激活模型。
    pub fn set_models(
        &mut self,
        models: Vec<super::input::ModelInfo>,
        active_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(input) = &self.input {
            input.update(cx, |input, cx| input.set_models(models, active_id, cx));
        }
    }

    /// 设置单条消息的扩展操作按钮。
    pub fn set_message_actions(
        &mut self,
        actions: Vec<MessageActionItem>,
        cx: &mut Context<Self>,
    ) {
        self.message_actions = actions.clone();
        if let Some(list) = &self.message_list {
            list.update(cx, |list, cx| list.set_custom_actions(actions, cx));
        }
    }

    /// 设置消息操作回调。
    ///
    /// 当用户对某条消息执行扩展操作（包括重新生成）时调用，
    /// 允许外部 ViewModel 在不必持有 `Subscription` 的情况下响应操作事件。
    pub fn set_on_message_action(
        &mut self,
        handler: impl Fn(&mut ChatPanel, u64, &ChatMessageAction, &mut Window, &mut Context<ChatPanel>)
            + 'static,
        cx: &mut Context<Self>,
    ) {
        self.on_message_action = Some(Rc::new(handler));
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

        let user_msg = ChatMessage::user(self.next_message_id, &content);
        self.next_message_id += 1;
        self.conversation.add_message(user_msg.clone());
        if let Some(list) = &self.message_list {
            list.update(cx, |list, cx| list.add_message(user_msg, cx));
        }

        let assistant_id = self.next_message_id;
        self.next_message_id += 1;
        let assistant_msg = ChatMessage::assistant(assistant_id, "").with_streaming(true);
        self.conversation.add_message(assistant_msg.clone());
        if let Some(list) = &self.message_list {
            list.update(cx, |list, cx| list.add_message(assistant_msg, cx));
        }

        self.stream_assistant_response(backend, ChatRequest::new(&content), cx);
    }

    /// 重新生成指定 AI 回复。
    ///
    /// 找到 `message_id` 对应的助手消息，剔除该回复及之后的上下文，
    /// 然后基于前一条用户消息重新向后端发起流式请求。
    pub fn regenerate(&mut self, message_id: u64, cx: &mut Context<Self>) {
        let Some(backend) = self.backend.clone() else {
            cx.emit(ChatEvent::Error("backend not configured".into()));
            return;
        };
        if self.is_streaming {
            return;
        }

        let assistant_index = self
            .conversation
            .messages
            .iter()
            .position(|m| m.id == message_id && m.role.is_assistant());
        let Some(assistant_index) = assistant_index else { return };
        if assistant_index == 0 {
            return;
        }
        let user_index = assistant_index - 1;
        if !self.conversation.messages[user_index].role.is_user() {
            return;
        }

        let user_content = self.conversation.messages[user_index].content.clone();
        self.conversation.truncate_from(assistant_index);
        if let Some(list) = &self.message_list {
            let messages = self.conversation.messages.clone();
            list.update(cx, |list, cx| list.set_messages(messages, cx));
        }

        let assistant_id = self.next_message_id;
        self.next_message_id += 1;
        let assistant_msg = ChatMessage::assistant(assistant_id, "").with_streaming(true);
        self.conversation.add_message(assistant_msg.clone());
        if let Some(list) = &self.message_list {
            list.update(cx, |list, cx| list.add_message(assistant_msg, cx));
        }

        self.stream_assistant_response(backend, ChatRequest::new(&user_content), cx);
    }

    /// 执行流式助手响应：将最后一条空助手消息填充为后端返回内容。
    fn stream_assistant_response(
        &mut self,
        backend: Arc<dyn IChatBackend>,
        request: ChatRequest,
        cx: &mut Context<Self>,
    ) {
        self.is_streaming = true;
        if let Some(input) = &self.input {
            input.update(cx, |input, cx| input.set_streaming(true, cx));
        }

        let conv = self.conversation.clone();
        cx.spawn(async move |this, cx| {
            let state: Arc<Mutex<StreamState>> = Arc::new(Mutex::new(StreamState {
                content: String::new(),
                thinking: String::new(),
                tool_calls: Vec::new(),
                attachments: Vec::new(),
            }));
            let state_clone = state.clone();

            let result = cx
                .background_executor()
                .spawn(async move {
                    backend.stream(&conv, &request, &move |event: &ChatStreamEvent| {
                        if let Ok(mut s) = state_clone.lock() {
                            match event {
                                ChatStreamEvent::Chunk(text) => s.content.push_str(text),
                                ChatStreamEvent::Thinking(text) => s.thinking.push_str(text),
                                ChatStreamEvent::ToolCall(tc) => s.tool_calls.push(tc.clone()),
                                ChatStreamEvent::Attachment(att) => s.attachments.push(att.clone()),
                                ChatStreamEvent::Done => {}
                            }
                        }
                    })
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                let s = state
                    .lock()
                    .map(|s| {
                        (
                            s.content.clone(),
                            s.thinking.clone(),
                            s.tool_calls.clone(),
                            s.attachments.clone(),
                        )
                    })
                    .unwrap_or_default();
                if let Some(list) = &this.message_list {
                    list.update(cx, |list, cx| list.update_last_message(&s.0, cx));
                }
                this.is_streaming = false;
                if let Some(input) = &this.input {
                    input.update(cx, |input, cx| input.set_streaming(false, cx));
                }
                match result {
                    Ok(msg) => cx.emit(ChatEvent::MessageReceived(msg)),
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
    pub fn messages(&self) -> &[ChatMessage] {
        &self.conversation.messages
    }

    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    /// 渲染 44px 头部栏（icon + title）。
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let title: SharedString = self.title.clone().into();

        div()
            .flex_shrink_0()
            .w_full()
            .h(px(44.))
            .flex()
            .flex_row()
            .items_center()
            .px(px(12.))
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::Bot)
                            .size(px(18.))
                            .text_color(theme.accent),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.foreground)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(title),
                    ),
            )
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
        let theme = cx.theme().clone();
        let input = self.input.clone();
        let message_list = self.message_list.clone();
        let header = self.render_header(cx);

        div()
            .id("chat-panel")
            .flex()
            .flex_col()
            .h_full()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .bg(theme.background)
            .text_color(theme.foreground)
            .track_focus(&self.focus_handle)
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .min_w_0()
                    .when_some(message_list, |el, list| el.child(list)),
            )
            .when_some(input, |el, input| el.child(input))
    }
}

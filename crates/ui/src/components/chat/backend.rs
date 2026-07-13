//! 通用聊天后端 trait。
//!
//! 替代源项目的 ACP（Agent Client Protocol）耦合，提供统一的聊天后端抽象：
//! - IM 场景：实现 [`IChatBackend::send`] 返回同步响应
//! - AI 场景：通过 [`IChatBackend::stream`] 返回流式响应，支持文本/思考/工具调用/附件事件

use super::model::{ChatAttachment, ChatConversation, ChatMessage, ChatRequest, ChatToolCall};

/// 聊天后端错误。
#[derive(Debug, Clone)]
pub enum ChatError {
    Network(String),
    Cancelled,
    Backend(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "network error: {}", msg),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Backend(msg) => write!(f, "backend error: {}", msg),
        }
    }
}

impl std::error::Error for ChatError {}

/// 流式事件 — 后端通过 `on_event` 回调推送的增量事件。
///
/// 支持文本增量、AI 思考过程、工具调用、附件等多种事件类型，
/// 使 UI 能实时渲染丰富的流式响应。
#[derive(Debug, Clone)]
pub enum ChatStreamEvent {
    /// 文本增量。
    Chunk(String),
    /// AI 思考过程增量。
    Thinking(String),
    /// 工具调用（完整推送，非增量）。
    ToolCall(ChatToolCall),
    /// 附件（如生成的图片、文件）。
    Attachment(ChatAttachment),
    /// 流式结束。
    Done,
}

/// 通用聊天后端 trait。
///
/// 实现此 trait 来对接不同的聊天服务：
/// - IM 后端：`send` 返回对方回复
/// - AI 后端：`stream` 通过回调推送增量事件（文本/思考/工具调用/附件）
pub trait IChatBackend: Send + Sync {
    /// 发送消息，返回完整响应。
    fn send(
        &self,
        conversation: &ChatConversation,
        request: &ChatRequest,
    ) -> Result<ChatMessage, ChatError>;

    /// 流式发送消息，通过 `on_event` 回调推送增量事件。
    ///
    /// 默认实现调用 `send` 并一次性推送 `Chunk` + `Done` 事件。
    fn stream(
        &self,
        conversation: &ChatConversation,
        request: &ChatRequest,
        on_event: &dyn Fn(&ChatStreamEvent),
    ) -> Result<ChatMessage, ChatError> {
        let message = self.send(conversation, request)?;
        on_event(&ChatStreamEvent::Chunk(message.content.clone()));
        on_event(&ChatStreamEvent::Done);
        Ok(message)
    }

    /// 取消当前请求。
    fn cancel(&self) -> Result<(), ChatError>;
}

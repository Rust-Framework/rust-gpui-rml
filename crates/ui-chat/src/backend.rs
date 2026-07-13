//! 通用聊天后端 trait。
//!
//! 替代源项目的 ACP（Agent Client Protocol）耦合，提供统一的聊天后端抽象：
//! - IM 场景：实现 [`ChatBackend::send_message`] 返回同步响应
//! - AI 场景：通过 [`ChatBackend::stream_message`] 返回流式响应

use crate::model::Conversation;

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

/// 通用聊天后端 trait。
///
/// 实现此 trait 来对接不同的聊天服务：
/// - IM 后端：`send_message` 返回对方回复
/// - AI 后端：`stream_message` 通过回调推送增量 token
///
/// # 示例
///
/// ```ignore
/// use rml_ui_chat::{ChatBackend, ChatError, Conversation};
///
/// struct EchoBackend;
///
/// impl ChatBackend for EchoBackend {
///     fn send_message(&self, _conv: &Conversation, content: &str) -> Result<String, ChatError> {
///         Ok(format!("echo: {}", content))
///     }
///     fn cancel(&self) -> Result<(), ChatError> { Ok(()) }
/// }
/// ```
pub trait ChatBackend: Send + Sync {
    /// 发送消息，返回完整响应。
    fn send_message(
        &self,
        conversation: &Conversation,
        content: &str,
    ) -> Result<String, ChatError>;

    /// 流式发送消息，通过 `on_chunk` 回调推送增量内容。
    ///
    /// 默认实现调用 `send_message` 并一次性推送完整响应。
    fn stream_message(
        &self,
        conversation: &Conversation,
        content: &str,
        on_chunk: &dyn Fn(&str),
    ) -> Result<(), ChatError> {
        let response = self.send_message(conversation, content)?;
        on_chunk(&response);
        Ok(())
    }

    /// 取消当前请求。
    fn cancel(&self) -> Result<(), ChatError>;
}

//! 聊天事件类型。

use super::action::ChatMessageAction;
use super::model::ChatMessage;

/// 聊天面板发出的事件。
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// 用户发送了一条消息
    MessageSent(String),
    /// 收到一条完整响应
    MessageReceived(ChatMessage),
    /// 收到流式文本增量
    StreamChunk(String),
    /// 流式响应结束
    StreamEnd,
    /// 发生错误
    Error(String),
    /// 请求被取消
    Cancelled,
    /// 用户选择了模型
    ModelSelected(String),
    /// 用户对某条消息执行了操作
    MessageAction {
        message_id: u64,
        action: ChatMessageAction,
    },
}

/// 聊天输入区域发出的事件。
#[derive(Debug, Clone)]
pub enum ChatInputEvent {
    Send(String),
    Stop,
    /// 用户通过模型选择器切换了模型
    SelectModel(String),
}

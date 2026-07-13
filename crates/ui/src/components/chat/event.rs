//! 聊天事件类型。

/// 聊天面板发出的事件。
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// 用户发送了一条消息
    MessageSent(String),
    /// 收到一条完整响应
    MessageReceived(String),
    /// 收到流式增量
    StreamChunk(String),
    /// 流式响应结束
    StreamEnd,
    /// 发生错误
    Error(String),
    /// 请求被取消
    Cancelled,
}

/// 聊天输入区域发出的事件。
#[derive(Debug, Clone)]
pub enum ChatInputEvent {
    Send(String),
    Stop,
}

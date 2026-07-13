//! 通用聊天消息模型。
//!
//! 提供兼容 IM 聊天与 AI 聊天的泛型消息类型：
//! - [`MessageRole`] 支持 User/Assistant/System/Custom，覆盖 IM（发送者/接收者）与 AI（用户/助手/系统）
//! - [`Message`] 带 metadata 扩展字段（thinking、tool_calls、attachments、streaming 状态）
//! - [`Conversation`] 管理消息列表与会话元数据

use serde::{Deserialize, Serialize};

/// 消息角色。
///
/// - IM 场景：User = 发送者，Assistant = 接收者，System = 系统通知
/// - AI 场景：User = 用户提问，Assistant = AI 回答，System = 系统提示词
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Custom(String),
}

impl MessageRole {
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User)
    }

    pub fn is_assistant(&self) -> bool {
        matches!(self, Self::Assistant)
    }
}

/// AI 工具调用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
}

/// 消息附件（IM/AI 通用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
}

/// 消息扩展元数据。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// AI 思考过程（仅 AI 场景）
    pub thinking: Option<String>,
    /// AI 工具调用列表（仅 AI 场景）
    pub tool_calls: Vec<ToolCall>,
    /// 附件列表（IM/AI 通用）
    pub attachments: Vec<Attachment>,
    /// 是否正在流式输出
    pub is_streaming: bool,
}

/// 单条聊天消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub role: MessageRole,
    pub content: String,
    pub timestamp_ms: u64,
    pub metadata: MessageMetadata,
}

impl Message {
    pub fn new(id: u64, role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            id,
            role,
            content: content.into(),
            timestamp_ms: now_ms(),
            metadata: MessageMetadata::default(),
        }
    }

    pub fn user(id: u64, content: impl Into<String>) -> Self {
        Self::new(id, MessageRole::User, content)
    }

    pub fn assistant(id: u64, content: impl Into<String>) -> Self {
        Self::new(id, MessageRole::Assistant, content)
    }

    pub fn system(id: u64, content: impl Into<String>) -> Self {
        Self::new(id, MessageRole::System, content)
    }

    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.metadata.thinking = Some(thinking.into());
        self
    }

    pub fn with_streaming(mut self, streaming: bool) -> Self {
        self.metadata.is_streaming = streaming;
        self
    }

    pub fn is_streaming(&self) -> bool {
        self.metadata.is_streaming
    }
}

/// 聊天会话。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: u64,
    pub title: String,
    pub messages: Vec<Message>,
    pub created_at_ms: u64,
}

impl Conversation {
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            messages: Vec::new(),
            created_at_ms: now_ms(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn last_message_mut(&mut self) -> Option<&mut Message> {
        self.messages.last_mut()
    }

    /// 根据第一条用户消息自动生成标题。
    pub fn auto_title(&mut self) {
        if let Some(msg) = self.messages.iter().find(|m| m.role.is_user()) {
            let s = msg.content.trim();
            self.title = if s.chars().count() > 30 {
                let preview: String = s.chars().take(30).collect();
                format!("{}...", preview)
            } else {
                s.to_string()
            };
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

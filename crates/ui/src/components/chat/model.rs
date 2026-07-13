//! 通用聊天消息模型。
//!
//! 提供兼容 IM 聊天与 AI 聊天的泛型消息类型：
//! - [`ChatRole`] 支持 User/Assistant/System/Custom，覆盖 IM（发送者/接收者）与 AI（用户/助手/系统）
//! - [`ChatMessage`] 带 metadata 扩展字段（thinking、tool_calls、attachments、streaming 状态）
//! - [`ChatConversation`] 管理消息列表与会话元数据
//! - [`ChatRequest`] 封装单次请求参数（内容 + 附件 + 配置覆盖）
//! - [`ChatConfig`] 封装对话级配置（模型、温度、max_tokens、系统提示）

use serde::{Deserialize, Serialize};

/// 消息角色。
///
/// - IM 场景：User = 发送者，Assistant = 接收者，System = 系统通知
/// - AI 场景：User = 用户提问，Assistant = AI 回答，System = 系统提示词
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChatRole {
    User,
    Assistant,
    System,
    Custom(String),
}

impl ChatRole {
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User)
    }

    pub fn is_assistant(&self) -> bool {
        matches!(self, Self::Assistant)
    }
}

/// AI 工具调用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
}

/// 消息附件（IM/AI 通用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAttachment {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
}

/// 消息扩展元数据。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatMetadata {
    /// AI 思考过程（仅 AI 场景）
    pub thinking: Option<String>,
    /// AI 工具调用列表（仅 AI 场景）
    pub tool_calls: Vec<ChatToolCall>,
    /// 附件列表（IM/AI 通用）
    pub attachments: Vec<ChatAttachment>,
    /// 是否正在流式输出
    pub is_streaming: bool,
}

/// 单条聊天消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: u64,
    pub role: ChatRole,
    pub content: String,
    pub timestamp_ms: u64,
    pub metadata: ChatMetadata,
}

impl ChatMessage {
    pub fn new(id: u64, role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            id,
            role,
            content: content.into(),
            timestamp_ms: now_ms(),
            metadata: ChatMetadata::default(),
        }
    }

    pub fn user(id: u64, content: impl Into<String>) -> Self {
        Self::new(id, ChatRole::User, content)
    }

    pub fn assistant(id: u64, content: impl Into<String>) -> Self {
        Self::new(id, ChatRole::Assistant, content)
    }

    pub fn system(id: u64, content: impl Into<String>) -> Self {
        Self::new(id, ChatRole::System, content)
    }

    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.metadata.thinking = Some(thinking.into());
        self
    }

    pub fn with_streaming(mut self, streaming: bool) -> Self {
        self.metadata.is_streaming = streaming;
        self
    }

    pub fn with_attachment(mut self, attachment: ChatAttachment) -> Self {
        self.metadata.attachments.push(attachment);
        self
    }

    pub fn is_streaming(&self) -> bool {
        self.metadata.is_streaming
    }
}

/// 聊天会话。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConversation {
    pub id: u64,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub created_at_ms: u64,
    /// 会话级配置（模型、温度等），可被单次 [`ChatRequest`] 覆盖。
    #[serde(default)]
    pub config: ChatConfig,
}

impl ChatConversation {
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            messages: Vec::new(),
            created_at_ms: now_ms(),
            config: ChatConfig::default(),
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn last_message(&self) -> Option<&ChatMessage> {
        self.messages.last()
    }

    pub fn last_message_mut(&mut self) -> Option<&mut ChatMessage> {
        self.messages.last_mut()
    }

    /// 从指定索引截断消息列表（移除该索引及之后的所有消息）。
    pub fn truncate_from(&mut self, index: usize) {
        self.messages.truncate(index);
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

/// 聊天会话配置（AI 场景）。
///
/// 存储在 [`ChatConversation::config`]，可被 [`ChatRequest::config`] 单次覆盖。
/// IM 后端可忽略这些字段。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatConfig {
    /// 模型标识（如 `"gpt-4"`、`"claude-3-opus"`）。
    #[serde(default)]
    pub model: Option<String>,
    /// 采样温度，越高越随机。
    #[serde(default)]
    pub temperature: Option<f64>,
    /// 最大生成 token 数。
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// 系统提示词。
    #[serde(default)]
    pub system_prompt: Option<String>,
}

/// 单次聊天请求。
///
/// 封装用户发送给后端的完整参数。`config` 为 `Some` 时覆盖会话级配置。
#[derive(Debug, Clone)]
pub struct ChatRequest {
    /// 文本内容。
    pub content: String,
    /// 附件列表。
    pub attachments: Vec<ChatAttachment>,
    /// 单次请求配置覆盖（`None` 时使用会话级配置）。
    pub config: Option<ChatConfig>,
}

impl ChatRequest {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            attachments: Vec::new(),
            config: None,
        }
    }

    pub fn with_attachment(mut self, attachment: ChatAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn with_config(mut self, config: ChatConfig) -> Self {
        self.config = Some(config);
        self
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

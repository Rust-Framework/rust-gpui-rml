//! 通用聊天组件模块。
//!
//! 提供通用聊天组件，支持 IM 聊天与 AI 聊天快速定制：
//! - IM 场景：实现 [`IChatBackend::send`] 返回同步响应
//! - AI 场景：通过 [`IChatBackend::stream`] 返回流式响应

pub mod action;
pub mod backend;
pub mod event;
pub mod input;
pub mod message_bubble;
pub mod message_list;
pub mod model;
pub mod panel;
pub mod renderer;

pub use action::{ChatMessageAction, MessageActionItem};
pub use backend::{ChatError, ChatStreamEvent, IChatBackend};
pub use event::{ChatEvent, ChatInputEvent};
pub use input::{ChatInput, ModelInfo};
pub use message_bubble::ChatBubble;
pub use message_list::{MessageListEvent, MessageListView};
pub use model::{
    ChatAttachment, ChatConfig, ChatConversation, ChatMessage, ChatMetadata, ChatRequest, ChatRole,
    ChatToolCall,
};
pub use panel::ChatPanel;
pub use renderer::{render_content, RenderMode};

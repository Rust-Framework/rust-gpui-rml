//! 聊天提供程序与工作台工厂 —— DefaultChatter / DefaultChatProvider / ChatWorkbenchProvider。
//!
//! - [`DefaultChatter`] —— IChatter 简单实现,持有 id/name/avatar 等元数据
//! - [`DefaultChatProvider`] —— IChatProvider 实现,提供 3 个演示 chatter
//! - [`ChatWorkbenchProvider`] —— IWorkbenchProvider(schema="chat"),构造 ChatWorkbench

use std::sync::Arc;

use gpui::SharedString;
use rml_core::contribution::{IContribution, IconSpec};
use rml_core::workbench::{IWorkbench, IWorkbenchProvider, Uri};
use studio_core::chat::{IChatProvider, IChatter};

// ──────────────────────────────────────────────────────────────────────────
//  DefaultChatter —— IChatter 简单实现
// ──────────────────────────────────────────────────────────────────────────

/// 演示用聊天对象 —— 持有 id/name/avatar/provider_id/kind/uri 元数据。
pub struct DefaultChatter {
    id: SharedString,
    name: SharedString,
    avatar: Option<IconSpec>,
    provider_id: SharedString,
    kind: SharedString,
    uri: SharedString,
}

impl DefaultChatter {
    /// 构造聊天对象,uri 自动按 `chat://{provider_id}/{id}` 格式生成。
    pub fn new(id: &str, name: &str, kind: &str, provider_id: &str) -> Self {
        let uri: SharedString = format!("chat://{provider_id}/{id}").into();
        Self {
            id: id.into(),
            name: name.into(),
            avatar: Some(IconSpec::named("User")),
            provider_id: provider_id.into(),
            kind: kind.into(),
            uri,
        }
    }
}

impl IChatter for DefaultChatter {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> SharedString {
        self.name.clone()
    }
    fn avatar(&self) -> Option<IconSpec> {
        self.avatar.clone()
    }
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn kind(&self) -> SharedString {
        self.kind.clone()
    }
    fn uri(&self) -> SharedString {
        self.uri.clone()
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  DefaultChatProvider —— IChatProvider 实现
// ──────────────────────────────────────────────────────────────────────────

/// 默认聊天提供程序 —— 提供 3 个演示 chatter(AI/群组/联系人)。
pub struct DefaultChatProvider {
    chatters: Vec<Arc<dyn IChatter>>,
}

impl Default for DefaultChatProvider {
    fn default() -> Self {
        let chatters: Vec<Arc<dyn IChatter>> = vec![
            Arc::new(DefaultChatter::new(
                "ai-assistant",
                "AI Assistant",
                "ai",
                "default",
            )),
            Arc::new(DefaultChatter::new(
                "team-group",
                "Team Group",
                "group",
                "default",
            )),
            Arc::new(DefaultChatter::new("john-doe", "John Doe", "im", "default")),
        ];
        Self { chatters }
    }
}

impl IContribution for DefaultChatProvider {
    fn id(&self) -> &str {
        "default-chat-provider"
    }
    fn name(&self) -> SharedString {
        "Default Chat".into()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("MessageCircle"))
    }
}

impl IChatProvider for DefaultChatProvider {
    fn provider_kind(&self) -> SharedString {
        "im".into()
    }
    fn chatters(&self) -> Vec<Arc<dyn IChatter>> {
        self.chatters.clone()
    }
}

/// 注册 DefaultChatProvider 能力 cast + 全局工厂。
pub fn register_default_chat_provider() {
    studio_core::chat::register_chat_provider_ability::<DefaultChatProvider>();
    studio_core::register_chat_provider(|| {
        Arc::new(DefaultChatProvider::default()) as Arc<dyn IChatProvider>
    });
}

// ──────────────────────────────────────────────────────────────────────────
//  ChatWorkbenchProvider —— IWorkbenchProvider(schema="chat")
// ──────────────────────────────────────────────────────────────────────────

/// `chat://` URI 的工作台工厂 —— 构造 ChatWorkbench。
pub struct ChatWorkbenchProvider;

impl IContribution for ChatWorkbenchProvider {
    fn id(&self) -> &str {
        "chat-workbench-provider"
    }
    fn name(&self) -> SharedString {
        "Chat Workbench Provider".into()
    }
}

impl IWorkbenchProvider for ChatWorkbenchProvider {
    fn schema(&self) -> SharedString {
        "chat".into()
    }

    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        let mut wb = crate::chat_workbench::ChatWorkbench::default();
        wb.set_uri(uri.as_str().into());
        Arc::new(wb)
    }
}

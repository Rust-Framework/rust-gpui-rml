//! ChatManager —— IChatManager 实现,聚合全局工厂发现的 IChatProvider。
//!
//! 经 DI 注册为 `dyn IChatManager` singleton,消费方经
//! `cx.get_service::<dyn IChatManager>()` 解析。
//!
//! MVP 阶段 Provider 经 `#[ctor::ctor]` + `register_chat_provider` 静态发现,
//! 运行时动态注册扩展点(IContributionHost)预留未实现。

use std::sync::Arc;

use studio_core::chat::{IChatManager, IChatProvider, IChatter};
use studio_core::get_chat_providers;

/// 聊天管理器 —— 聚合所有已注册 IChatProvider,提供统一 IChatter 查询。
pub struct ChatManager {
    providers: Vec<Arc<dyn IChatProvider>>,
}

impl ChatManager {
    /// 从全局工厂注册表加载所有已注册 Provider。
    pub fn new() -> Self {
        Self {
            providers: get_chat_providers(),
        }
    }
}

impl Default for ChatManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IChatManager for ChatManager {
    fn providers(&self) -> Vec<Arc<dyn IChatProvider>> {
        self.providers.clone()
    }

    fn chatters(&self) -> Vec<Arc<dyn IChatter>> {
        self.providers.iter().flat_map(|p| p.chatters()).collect()
    }

    fn find_chatter(&self, uri: &str) -> Option<Arc<dyn IChatter>> {
        self.chatters().into_iter().find(|c| c.uri() == uri)
    }
}

//! ChatComponent ViewModel —— IWorkbenchComponent,复用现成 `rml_ui::ChatPanel`。
//!
//! 经 RML `<Chat ref="chat" />` EntityRef 标签渲染现成 ChatPanel(GPUI View),
//! ChatPanel 内置 44px 头部栏 + MessageListView(消息列表)+ ChatInput(输入区),
//! 并经 `IChatBackend` trait 对接不同聊天后端(IM 同步 / AI 流式)。
//!
//! # 数据同步链路
//!
//! ```text
//! ChatWorkbench.reload → document.reload(新 uri) → ILifecycle::before_render
//!   → ChatComponent 经 get_or_create_entity::<ChatWorkbench> 读 chatter_name
//!   → panel.set_title(chatter_name) 更新 ChatPanel 头部
//! ```
//!
//! # 后端注入
//!
//! MVP 阶段使用 `EchoChatBackend`(回显用户消息)。后续可按 `IChatter.kind()`
//! 注入不同 `IChatBackend` 实现:
//! - kind="ai" → AI 流式后端(经 `stream()` 推送增量事件)
//! - kind="im" → IM 同步后端(经 `send()` 返回完整响应)
//! - kind="email" → 邮件会话后端

use std::sync::Arc;

use gpui::{Entity, SharedString, Window};
use rml::prelude::*;
use rml_app::contribution::get_active_entity;
use rml_core::contribution::{IconSpec, IContribution};
use rml_core::workbench::Uri;
use rml_ui::{
    ChatConversation, ChatError, ChatMessage, ChatPanel, ChatRequest, IChatBackend, RenderMode,
};
use studio_core::ability_ext::register_workbench_component_ability;
use studio_core::component::IWorkbenchComponent;
use studio_core::register_workbench_component;

use crate::chat_workbench::ChatWorkbench;

/// Echo 聊天后端 —— MVP 回显后端,将用户消息原样返回(含 Echo 前缀)。
///
/// 参照 demo `chat_case.rml.rs` 的 EchoBackend 实现。
/// 后续迭代可按 IChatter.kind() 替换为真实 AI/IM/Email 后端。
struct EchoChatBackend;

impl IChatBackend for EchoChatBackend {
    fn send(
        &self,
        _conv: &ChatConversation,
        request: &ChatRequest,
    ) -> Result<ChatMessage, ChatError> {
        let reply = format!("Echo: {}", request.content);
        Ok(ChatMessage::assistant(0, reply))
    }

    fn cancel(&self) -> Result<(), ChatError> {
        Ok(())
    }
}

/// 聊天交互视图组件 —— 复用现成 `rml_ui::ChatPanel` 经 `<Chat ref="chat" />` 渲染。
///
/// `#[component]` 生成 RML 框架契约(IModel/IViewModel/IComponent/IVisual/Render),
/// 经 `include!` 引入编译器生成的 `impl Render` 驱动 `.rml` 模板。
///
/// 手动 impl:
/// - `IContribution` —— 元数据(id/name/icon)
/// - `ILifecycle` —— on_loaded 创建 ChatPanel Entity + 注入 EchoChatBackend + set_title;
///   before_render 同步 host chatter_name 变化(Tab 切换)
/// - `IWorkbenchComponent` —— `matches(uri)` 仅匹配 `chat://` scheme
#[component]
#[derive(Default)]
pub struct ChatComponent {
    /// 现成 ChatPanel Entity —— 经 RML `<Chat ref="chat" />` EntityRef 渲染。
    ///
    /// 在 `on_loaded` 中创建,经 `set_backend` 注入 EchoChatBackend,
    /// 经 `set_title` 设置 chatter 名称(从 host ChatWorkbench 读取)。
    pub chat: Option<Entity<ChatPanel>>,
    /// 上次同步到 ChatPanel 的标题,避免重复 set_title 触发 cx.notify 循环。
    last_synced_title: SharedString,
}

impl IContribution for ChatComponent {
    fn id(&self) -> &str {
        "chat"
    }
    fn name(&self) -> SharedString {
        "Chat".into()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("MessageCircle"))
    }
}

impl ILifecycle for ChatComponent {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 创建 ChatPanel Entity(Markdown 渲染模式,支持 GFM 富文本)
        let chat = cx.new(|cx| ChatPanel::new(RenderMode::Markdown, window, cx));

        // 从 host(ChatWorkbench)获取 chatter_name + 注入 backend
        let title = {
            let host = match get_active_entity::<ChatWorkbench>(cx) {
                Some(h) => h,
                None => return, // host 尚未渲染(非 ChatWorkbench 上下文),跳过初始化
            };
            let chatter_name = host.read(cx).chatter_name.clone();
            if chatter_name.is_empty() {
                "Chat".to_string()
            } else {
                chatter_name.to_string()
            }
        };
        chat.update(cx, |panel, cx| {
            panel.set_title(title.clone(), cx);
            // 注入 EchoChatBackend(MVP:回显后端)
            panel.set_backend(Arc::new(EchoChatBackend) as Arc<dyn IChatBackend>, cx);
        });
        self.last_synced_title = title.into();

        self.chat = Some(chat);
    }

    fn before_render(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 同步 host chatter_name 变化(Tab 切换时 ChatWorkbench.reload 更新 chatter_name)
        self.sync_from_host(cx);
    }
}

impl IWorkbenchComponent for ChatComponent {
    fn matches(&self, uri: &Uri) -> bool {
        // 仅匹配 chat:// scheme,避免出现在 EditorWorkbench 的 file:// 视图中
        uri.scheme() == "chat"
    }
}

impl ChatComponent {
    /// 从 host(ChatWorkbench)同步 chatter_name 到 ChatPanel 头部标题。
    ///
    /// 在 `ILifecycle::before_render` 中每帧调用。Tab 切换时 ChatWorkbench.reload 更新
    /// chatter_name,此处经 `last_synced_title` 比对避免重复 set_title 触发循环。
    fn sync_from_host(&mut self, cx: &mut Context<Self>) {
        let host = match get_active_entity::<ChatWorkbench>(cx) {
            Some(h) => h,
            None => return, // host 尚未渲染(非 ChatWorkbench 上下文),跳过
        };
        let chatter_name = host.read(cx).chatter_name.clone();
        let title = if chatter_name.is_empty() {
            return; // host 尚未初始化,跳过
        } else {
            chatter_name
        };

        // 经 last_synced_title 比对,避免重复 set_title 触发 cx.notify 循环
        if self.last_synced_title != title {
            self.last_synced_title = title.clone();
            if let Some(chat) = self.chat.as_ref() {
                chat.update(cx, |panel, cx| {
                    panel.set_title(title.to_string(), cx);
                });
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:ChatComponent 需注册 IWorkbenchComponent 能力 cast + 工厂。
// ──────────────────────────────────────────────────────────────────────────

/// 注册 ChatComponent 能力 cast + 工厂。
///
/// 在 `#[ctor::ctor]` 中调用:
/// 1. `register_workbench_component_ability::<ChatComponent>()` —— 注册能力 cast
/// 2. `register_workbench_component(factory)` —— 注册工厂到全局注册表
pub fn register_chat_component() {
    register_workbench_component_ability::<ChatComponent>();
    register_workbench_component(|| {
        Arc::new(ChatComponent::default()) as Arc<dyn IWorkbenchComponent>
    });
}

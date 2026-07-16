//! 聊天契约 —— IChatter / IChatProvider / IChatManager + 能力扩展。
//!
//! 三接口分工:
//! - [`IChatter`] —— 聊天对象接口(联系人/群组/AI Agent/邮件会话等)。
//! - [`IChatProvider`] —— 聊天支持贡献点提供程序,impl `IContribution`(具备元数据),
//!   提供该来源(邮件/IM/AI 智能体等)的 `IChatter` 集合。
//! - [`IChatManager`] —— 公共聊天管理器接口,注册到全局 DI 容器(面向接口),
//!   聚合所有 `IChatProvider`,提供统一的 `IChatter` 查询能力。
//!
//! # 注册路径
//!
//! - **静态发现**:各 Provider crate 经 `#[ctor::ctor]` + `register_chat_provider(factory)`
//!   注册到全局工厂注册表;`ChatManager::new()` 从 `get_chat_providers()` 加载。
//! - **运行时扩展**(预留):`ChatManager` impl `IContributionHost`,经贡献注册表
//!   接受插件动态注册的 `IChatProvider`。

use std::sync::Arc;

use gpui::SharedString;
use rml_core::contribution::{IContribution, IconSpec};
use rml_core::value::IValue;

/// 聊天对象接口 —— 代表一个可聊天的对象(联系人/群组/AI Agent/邮件会话等)。
///
/// `IChatter: IValue`——聊天对象是值对象的特化,经 `Arc<dyn IChatter>` 在
/// `IChatManager` / `ChatPanel` / `ChatWorkbench` 间传递。
pub trait IChatter: IValue {
    /// 聊天对象唯一标识(在 Provider 范围内唯一)。
    fn id(&self) -> &str;
    /// 显示名称(联系人名/群名/AI Agent 名等)。
    fn name(&self) -> SharedString;
    /// 头像图标规格。返回 `None` 时由框架走 fallback。
    fn avatar(&self) -> Option<IconSpec>;
    /// 所属 Provider 的 id(对应 [`IChatProvider::id`])。
    fn provider_id(&self) -> &str;
    /// 聊天对象类型(开放字符串):`"im"` / `"email"` / `"ai"` / `"group"` 等。
    fn kind(&self) -> SharedString;
    /// 聊天资源 URI:`"chat://{provider_id}/{chatter_id}"`。
    ///
    /// `ChatWorkbench` 经此 URI 打开工作台,`ChatComponent` 经 `matches(uri)` 适配。
    fn uri(&self) -> SharedString;
}

/// 聊天支持贡献点提供程序 —— 可获得 `IChatter` 集合的数据源。
///
/// 继承 `IContribution`(具备 `id`/`name`/`icon` 元数据,纳入贡献体系)。
/// 每种聊天来源(邮件/IM/AI 智能体等)实现此 trait,经 `register_chat_provider` 工厂注册。
///
/// # 示例
///
/// - `DefaultChatProvider`(provider_kind="im")—— 内置 IM 聊天源
/// - `EmailChatProvider`(provider_kind="email")—— 邮件会话源
/// - `AIChatProvider`(provider_kind="ai")—— AI 智能体源
pub trait IChatProvider: IContribution {
    /// Provider 类型标识(开放字符串):`"email"` / `"im"` / `"ai"` 等。
    fn provider_kind(&self) -> SharedString;
    /// 此 Provider 提供的聊天对象集合。
    fn chatters(&self) -> Vec<Arc<dyn IChatter>>;
}

/// 公共聊天管理器接口 —— 注册到全局 DI 容器(面向接口)。
///
/// 聚合所有 `IChatProvider`,提供统一的 `IChatter` 查询能力。
/// 业务实现此 trait(`ChatManager`),经 `add_singleton::<dyn IChatManager>` 注册到 DI,
/// 消费方经 `cx.get_service::<dyn IChatManager>()` 解析。
///
/// # 扩展性
///
/// `ChatManager` 实现 `IContributionHost`,支持运行时经贡献注册表动态注册
/// `IChatProvider`(插件扩展场景)。MVP 阶段 Provider 经全局工厂静态发现。
pub trait IChatManager: Send + Sync + 'static {
    /// 所有已注册的 `IChatProvider`。
    fn providers(&self) -> Vec<Arc<dyn IChatProvider>>;
    /// 聚合所有 Provider 的 `IChatter` 集合。
    fn chatters(&self) -> Vec<Arc<dyn IChatter>>;
    /// 按 URI 查找 `IChatter`(`"chat://provider_id/chatter_id"`)。
    fn find_chatter(&self, uri: &str) -> Option<Arc<dyn IChatter>>;
}

// ──────────────────────────────────────────────────────────────────────────
//  ChatProvider 能力扩展 —— 让 dyn IValue / dyn IContribution 可查询 IChatProvider
// ──────────────────────────────────────────────────────────────────────────

/// 聊天提供程序能力扩展 —— 让 `dyn IValue` 可查询 `IChatProvider` 能力。
///
/// 与 `WorkbenchComponentAbilityExt` 模式一致。`IChatProvider` 实现类型经
/// `register_chat_provider_ability::<T>()` 注册后,`as_chat_provider()` 查询生效。
pub trait ChatProviderAbilityExt {
    /// 若此值实现了 `IChatProvider`,返回引用;否则 `None`。
    fn as_chat_provider(&self) -> Option<&dyn IChatProvider>;
}

#[allow(unsafe_code)]
impl ChatProviderAbilityExt for dyn IValue {
    fn as_chat_provider(&self) -> Option<&dyn IChatProvider> {
        let erased = rml_core::ability::query::<dyn IChatProvider>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IChatProvider>(erased) })
    }
}

/// `dyn IContribution` 薄委托——trait upcast 到 `&dyn IValue` 后调用主 impl,
/// 使注册表返回的 `Arc<dyn IContribution>` 可直接调用 `as_chat_provider()`。
impl ChatProviderAbilityExt for dyn IContribution {
    fn as_chat_provider(&self) -> Option<&dyn IChatProvider> {
        let iv: &dyn IValue = self;
        iv.as_chat_provider()
    }
}

/// 为实现 `IChatProvider` 的类型注册能力 cast 函数。
///
/// 业务自定义 Provider 类型后,需在 `#[ctor::ctor]` 中调用此函数注册,
/// 使 `as_chat_provider()` 查询生效,`ChatManager` 可据此分类受理。
#[allow(unsafe_code)]
pub fn register_chat_provider_ability<T: IChatProvider + 'static>() {
    rml_core::ability::register::<T, dyn IChatProvider>(|c| {
        let any: &dyn std::any::Any = c;
        any.downcast_ref::<T>().map(|s| {
            let p: &dyn IChatProvider = s;
            unsafe { rml_core::ability::erase(p) }
        })
    });
}

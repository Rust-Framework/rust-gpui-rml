//! ChatterItem —— 聊天列表项数据结构。
//!
//! 原 ChatListItem 子组件方案(RML `<ChatListItem each={...} />`)不可行:
//! RML 用户组件(PascalCase 标签)不支持 `each` 指令 —— 始终作为 EntityRef 单例渲染。
//! 改为在 chat_panel.rml 中内联列表项渲染,经 `on-click={open_chatter(item.uri)}`
//! (WithArgs 模式)将 URI 传递给 ChatPanel::open_chatter 方法。
//!
//! 此文件仅保留 `ChatterItem` 数据结构,供 ChatPanel 聚合 IChatter 后构建列表。

use gpui::SharedString;

/// 聊天列表项数据(普通 struct,非 `#[component]`)。
///
/// 由 ChatPanel 从 `IChatManager` 聚合的 `IChatter` 构建而成,
/// 经 `each={item in filtered_list}` 迭代渲染。
#[derive(Clone, Default)]
pub struct ChatterItem {
    /// 聊天对象唯一标识。
    pub id: SharedString,
    /// 显示名称(联系人名/群名/AI Agent 名等)。
    pub name: SharedString,
    /// 名称首字符(头像占位文字)。
    pub initial: SharedString,
    /// 聊天对象类型("im" / "ai" / "group" 等)。
    pub kind: SharedString,
    /// 聊天资源 URI:`"chat://{provider_id}/{chatter_id}"`。
    pub uri: SharedString,
    /// 最后一条消息预览(MVP 占位:"开始对话...")。
    pub last_message: SharedString,
    /// 最后消息时间(MVP 占位:"")。
    pub time: SharedString,
    /// 未读消息数(MVP 占位:0)。
    pub unread: u32,
}

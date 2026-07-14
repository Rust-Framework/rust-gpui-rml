//! ChatListItem ViewModel —— 微信风格聊天列表项子组件。
//!
//! 解决 RML `each` + `on-click` 不传递 item 上下文的问题:
//! 每个列表项是独立的 RML 子组件,持有自己的 `ChatterItem` 数据,
//! `#[command] on_click` 经 `get_or_create_entity::<ChatPanel>` 回调宿主。
//!
//! 此模式与 CodeComponent 经 `get_or_create_entity::<EditorWorkbench>` 获取宿主一致。

use gpui::SharedString;
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;

use crate::chat_panel::ChatPanel;

/// 聊天列表项数据(普通 struct,非 `#[component]`)。
///
/// 由 ChatPanel 从 `IChatManager` 聚合的 `IChatter` 构建而成,
/// 经 `<ChatListItem each={item in filtered_list} item={item} />` 传递给子组件。
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

/// 微信风格聊天列表项子组件。
///
/// `#[component]` 生成 RML 框架契约(IModel/IViewModel/IComponent/Render),
/// 经 `include!` 引入 RML 编译器生成的 `impl Render` 驱动 `.rml` 模板。
///
/// 点击列表项时,经 `get_or_create_entity::<ChatPanel>` 获取宿主 Entity,
/// 调用 `ChatPanel::open_chatter(uri)` 打开对应聊天工作台。
#[component]
#[derive(Default)]
pub struct ChatListItem {
    /// 此项绑定的聊天对象数据。
    pub item: ChatterItem,
}

impl ChatListItem {
    /// 点击列表项:经 `get_or_create_entity` 获取 ChatPanel 宿主,调用 `open_chatter`。
    ///
    /// 此模式解决了 `each` + `on-click` 不传递 item 上下文的问题:
    /// ChatListItem 从自身 `item.uri` 字段获取聊天对象 URI,无需外部传入。
    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let uri = self.item.uri.clone();
        let panel = get_or_create_entity::<ChatPanel>(cx);
        panel.update(cx, |panel, ctx| {
            panel.open_chatter(uri, ctx);
        });
    }

    /// 未读消息数文本(用于角标渲染)。
    #[computed]
    pub fn unread_text(&self) -> SharedString {
        self.item.unread.to_string().into()
    }
}

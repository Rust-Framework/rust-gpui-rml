//! ChatPanel ViewModel —— 微信风格活动栏聊天列表面板。
//!
//! `#[component]` RML 组件,作为 ActivityBar 面板贡献:
//! - 手动 `impl IContribution`(id/name/icon 元数据)
//! - `impl ILifecycle::on_loaded` → 从 DI 获取 `IChatManager` → 构建 `chatter_list`
//! - `open_chatter(uri)` —— 由 ChatListItem 经 `get_active_entity` 回调
//!
//! 经 DI 获取 `IChatManager` 聚合所有 `IChatProvider` 的 `IChatter` 集合,
//! 渲染为微信风格聊天列表。点击列表项 → `IWorkbenchManager::open(chat_uri)`。

use std::sync::Once;

use gpui::{SharedString, Window};
use rml::prelude::*;
use rml_app::IServiceProvider;
use rml_core::contribution::{
    IconSpec, IContribution, register_contribution_ability, register_visual_ability,
};
use rml_core::workbench::IWorkbenchManager;

use crate::chat_list_item::ChatterItem;

/// Arc Studio 聊天面板 —— 微信风格活动栏贡献。
///
/// `#[component]` 生成 `impl IModel + IViewModel + IComponent + IVisual`(RML 框架契约),
/// 经 `include!` 引入 RML 编译器生成的 `impl Render` 驱动 `.rml` 模板。
///
/// 手动 `impl IContribution + ILifecycle` 补充元数据 + 生命周期
/// (因 `#[contribute]` 被项目规范拒绝 —— 生成 `contribution_entries` 污染业务代码)。
#[component]
#[derive(Default)]
pub struct ChatPanel {
    /// 全部聊天对象列表(从 IChatManager 聚合)。
    chatter_list: Vec<ChatterItem>,
    /// 当前选中项 id(MVP 阶段仅记录,不渲染选中态)。
    selected_id: SharedString,
}

impl IContribution for ChatPanel {
    fn id(&self) -> &str {
        "chat-panel"
    }
    fn name(&self) -> SharedString {
        "Chat".into()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("MessageCircle"))
    }
}

impl ILifecycle for ChatPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_chatters(cx);
    }
}

impl ChatPanel {
    /// 刷新聊天列表:经 DI 获取 IChatManager → 聚合 IChatter → 构建 ChatterItem。
    fn refresh_chatters(&mut self, cx: &mut Context<Self>) {
        let chatters = cx
            .get_service::<dyn studio_core::chat::IChatManager>()
            .map(|mgr| mgr.chatters())
            .unwrap_or_default();

        self.chatter_list = chatters
            .iter()
            .map(|c| ChatterItem {
                id: c.id().into(),
                name: c.name(),
                initial: c
                    .name()
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_string()
                    .into(),
                kind: c.kind(),
                uri: c.uri(),
                last_message: "开始对话...".into(),
                time: "".into(),
                unread: 0,
            })
            .collect();
        cx.notify();
    }

    /// 由列表项 `on-click={open_chatter, item.uri}` (WithArgs 模式) 回调。
    ///
    /// 解析 URI → `IWorkbenchManager::open(uri)` 打开 ChatWorkbench Tab。
    pub fn open_chatter(&mut self, uri: SharedString, _ev: &ClickEvent, cx: &mut Context<Self>) {
        // 更新选中态
        if let Some(item) = self.chatter_list.iter().find(|i| i.uri == uri) {
            self.selected_id = item.id.clone();
        }
        // 解析 URI → IWorkbenchManager::open
        if let Ok(parsed) = uri.parse::<rml_core::workbench::Uri>() {
            if let Some(mgr) = cx.get_service::<dyn IWorkbenchManager>() {
                mgr.open(&parsed);
            }
        }
        cx.notify();
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:ChatPanel 需注册 IContribution + IVisual 能力 cast,
//  使 VisualActivityPanel::new(c).as_visual() 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

pub fn register_chat_panel_abilities() {
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<ChatPanel>();
        register_visual_ability::<ChatPanel>();
    });
}

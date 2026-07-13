//! 消息交互动作枚举与扩展定义。

use gpui::SharedString;
use gpui_component::IconName;

/// 可对单条聊天消息执行的操作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatMessageAction {
    /// 复制消息内容到剪贴板（内置，组件自身处理）。
    Copy,
    /// 扩展操作，由外部 ViewModel 通过 [`ChatEvent::MessageAction`] 处理。
    Custom(SharedString),
}

/// 扩展消息操作按钮定义。
///
/// 通过 [`ChatPanel::set_message_actions`] 注入，渲染在每条非系统消息的复制按钮之后。
#[derive(Clone)]
pub struct MessageActionItem {
    /// 操作唯一标识，点击时以 `ChatMessageAction::Custom(id)` 形式发出。
    pub id: SharedString,
    /// 按钮图标。
    pub icon: IconName,
    /// 悬停提示文案。
    pub tooltip: SharedString,
}

impl MessageActionItem {
    pub fn new(
        id: impl Into<SharedString>,
        icon: IconName,
        tooltip: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            tooltip: tooltip.into(),
        }
    }
}

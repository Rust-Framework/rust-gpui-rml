//! 单条消息气泡组件。
//!
//! 视觉规范对齐源项目 `agent/src/chat/message_list_view.rs`：
//! - User：右对齐，bg=background，rounded_lg，max_w=480
//! - Assistant：左对齐，24x24 Bot 图标 + 内容，流式光标（2x14px accent 竖条）
//! - System：居中，muted bg 药丸，text_xs
//! - 操作按钮：Copy + 时间戳

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::{ActiveTheme, Icon, IconName, Sizable as _};

use super::action::{ChatMessageAction, MessageActionItem};
use super::message_list::{MessageListEvent, MessageListView};
use super::model::{ChatMessage, ChatRole};
use super::renderer::{render_content, RenderMode};

const BUBBLE_MAX_WIDTH: Pixels = px(480.);
const BUBBLE_GROUP: &str = "chat-bubble";

/// 消息气泡。
#[derive(IntoElement)]
pub struct ChatBubble {
    message: ChatMessage,
    render_mode: RenderMode,
    message_list: Entity<MessageListView>,
    custom_actions: Vec<MessageActionItem>,
}

impl ChatBubble {
    pub fn new(
        message: ChatMessage,
        render_mode: RenderMode,
        message_list: &Entity<MessageListView>,
        custom_actions: Vec<MessageActionItem>,
    ) -> Self {
        Self {
            message,
            render_mode,
            message_list: message_list.clone(),
            custom_actions,
        }
    }
}

impl RenderOnce for ChatBubble {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let role = self.message.role.clone();
        let content_str = self.message.content.clone();
        let is_streaming = self.message.is_streaming();
        let timestamp = format_timestamp(self.message.timestamp_ms);

        let custom_actions = self.custom_actions.clone();

        match role {
            ChatRole::User => render_user_message(
                &self,
                &theme,
                &content_str,
                &timestamp,
                self.message_list.clone(),
                self.message.id,
                custom_actions,
                window,
                cx,
            )
            .into_any_element(),
            ChatRole::System => {
                render_system_message(&theme, &content_str, window, cx).into_any_element()
            }
            _ => render_assistant_message(
                &self,
                &theme,
                &content_str,
                &timestamp,
                is_streaming,
                self.message_list.clone(),
                self.message.id,
                custom_actions,
                window,
                cx,
            )
            .into_any_element(),
        }
    }
}

/// 用户消息：右对齐，accent 背景气泡 + 操作按钮 + 时间戳
fn render_user_message(
    this: &ChatBubble,
    theme: &gpui_component::Theme,
    content: &str,
    timestamp: &str,
    message_list: Entity<MessageListView>,
    message_id: u64,
    custom_actions: Vec<MessageActionItem>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let bubble_content = render_content(this.render_mode, content, window, cx);

    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .items_end()
        .group(BUBBLE_GROUP)
        .mb(px(12.))
        .child(
            div()
                .min_w_0()
                .max_w(BUBBLE_MAX_WIDTH)
                .overflow_hidden()
                .bg(theme.accent.opacity(0.12))
                .rounded_lg()
                .rounded_tr(px(4.))
                .px(px(12.))
                .py(px(8.))
                .text_sm()
                .text_color(theme.foreground)
                .whitespace_normal()
                .child(bubble_content),
        )
        .child(render_message_actions(
            content,
            timestamp,
            theme,
            message_list,
            message_id,
            custom_actions,
            cx,
        ))
}

/// AI 消息：Bot 图标 + 内容 + 流式光标 + 操作按钮 + 时间戳
fn render_assistant_message(
    this: &ChatBubble,
    theme: &gpui_component::Theme,
    content: &str,
    timestamp: &str,
    is_streaming: bool,
    message_list: Entity<MessageListView>,
    message_id: u64,
    custom_actions: Vec<MessageActionItem>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let has_content = !content.is_empty();
    let bubble_content = if has_content {
        Some(render_content(this.render_mode, content, window, cx))
    } else {
        None
    };

    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .items_start()
        .group(BUBBLE_GROUP)
        .mb(px(12.))
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_row()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .flex_shrink_0()
                        .w(px(24.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .bg(theme.accent.opacity(0.1))
                        .child(
                            Icon::new(IconName::Bot)
                                .with_size(px(14.))
                                .text_color(theme.accent),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .flex_row()
                                .items_start()
                                .gap_1()
                                .overflow_hidden()
                                .bg(theme.secondary.opacity(0.12))
                                .rounded_lg()
                                .rounded_tl(px(4.))
                                .px(px(12.))
                                .py(px(8.))
                                .when_some(bubble_content, |el, content| {
                                    el.child(div().flex_1().min_w_0().child(content))
                                })
                                .when(is_streaming, |el| {
                                    el.child(
                                        div()
                                            .flex_shrink_0()
                                            .mt(px(4.))
                                            .w(px(2.))
                                            .h(px(14.))
                                            .bg(theme.accent),
                                    )
                                }),
                        )
                        .when(!is_streaming && has_content, |el| {
                            el.child(render_message_actions(
                                content,
                                timestamp,
                                theme,
                                message_list,
                                message_id,
                                custom_actions,
                                cx,
                            ))
                        }),
                ),
        )
}

/// 系统消息：居中，muted bg 药丸
fn render_system_message(
    theme: &gpui_component::Theme,
    content: &str,
    _window: &mut Window,
    _cx: &mut App,
) -> impl IntoElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_row()
        .justify_center()
        .mb(px(4.))
        .child(
            div()
                .w_full()
                .min_w_0()
                .overflow_hidden()
                .px(px(8.))
                .py(px(2.))
                .rounded_md()
                .bg(theme.muted)
                .text_xs()
                .text_color(theme.muted_foreground)
                .whitespace_normal()
                .child(content.to_string()),
        )
}

/// 操作按钮行：复制（内置）+ 扩展操作 + 时间戳。
fn render_message_actions(
    content: &str,
    timestamp: &str,
    theme: &gpui_component::Theme,
    message_list: Entity<MessageListView>,
    message_id: u64,
    custom_actions: Vec<MessageActionItem>,
    _cx: &mut App,
) -> impl IntoElement {
    let content = content.to_string();
    let emit = move |action: ChatMessageAction, cx: &mut App| {
        message_list.update(cx, |_, cx| {
            cx.emit(MessageListEvent::MessageAction { message_id, action });
        });
    };

    let mut actions_row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .invisible()
        .group_hover(BUBBLE_GROUP, |this| this.visible())
        .child(
            Button::new(SharedString::from(format!("copy-{message_id}")))
                .ghost()
                .cursor_pointer()
                .w(px(22.))
                .h(px(22.))
                .px(px(4.))
                .rounded_sm()
                .icon(IconName::Copy)
                .tooltip("复制")
                .on_click({
                    let content = content.clone();
                    move |_, _, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(content.clone()));
                    }
                }),
        );

    for action in custom_actions {
        let emit = emit.clone();
        let id = action.id.clone();
        actions_row = actions_row.child(
            Button::new(SharedString::from(format!("{}-{message_id}", action.id)))
                .ghost()
                .cursor_pointer()
                .w(px(22.))
                .h(px(22.))
                .px(px(4.))
                .rounded_sm()
                .icon(action.icon)
                .tooltip(action.tooltip.clone())
                .on_click(move |_, _, cx| emit(ChatMessageAction::Custom(id.clone()), cx)),
        );
    }

    actions_row.child(
        div()
            .ml(px(4.))
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(timestamp.to_string()),
    )
}

/// 格式化时间戳为相对时间。
fn format_timestamp(ts_ms: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let diff_secs = now.saturating_sub(ts_ms) / 1000;
    if diff_secs < 60 {
        return "刚刚".to_string();
    }
    if diff_secs < 3600 {
        return format!("{} 分钟前", diff_secs / 60);
    }
    if diff_secs < 86400 {
        return format!("{} 小时前", diff_secs / 3600);
    }
    format!("{} 天前", diff_secs / 86400)
}

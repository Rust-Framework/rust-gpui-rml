//! NotificationTrigger —— 声明式通知触发器
//!
//! RML `<Notification>` 标签编译为 `NotificationTrigger`，而非直接使用 gpui-component 的 `Notification`。
//!
//! ## 设计原因
//!
//! `Notification` 是 `Render`（非 `RenderOnce`），且通过 `window.push_notification()` 命令式推送。
//! RML 需要声明式触发：`<Notification>` 包裹一个 `slot="trigger"` 子元素，点击时自动推送通知。
//!
//! ## 构造模式
//!
//! ```ignore
//! NotificationTrigger::new()
//!     .title("保存成功")
//!     .message("您的更改已保存")
//!     .with_type(NotificationType::Success)
//!     .trigger(Button::new("save").label("Save"))
//! ```
//!
//! 点击 trigger 时，构造 `Notification` 并调用 `window.push_notification()` 推送。

use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, div,
};
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::StyledExt as _;

use crate::WindowExt as _;

/// 声明式通知触发器
///
/// 包裹一个 trigger 元素，点击时自动推送通知。
/// 通知字段（title/message/type/autohide）在 render 时捕获到闭包中，
/// 每次点击构造新的 `Notification` 并推送（因 `Fn` 闭包不可消费 Notification）。
#[derive(IntoElement)]
pub struct NotificationTrigger {
    title: Option<SharedString>,
    message: Option<SharedString>,
    type_: NotificationType,
    autohide: bool,
    trigger: Option<AnyElement>,
    style: StyleRefinement,
}

impl Default for NotificationTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationTrigger {
    /// 创建通知触发器，默认类型为 Info
    pub fn new() -> Self {
        Self {
            title: None,
            message: None,
            type_: NotificationType::Info,
            autohide: true,
            trigger: None,
            style: StyleRefinement::default(),
        }
    }

    /// 设置通知标题
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 设置通知消息
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// 设置通知类型（Info/Success/Warning/Error）
    pub fn with_type(mut self, type_: NotificationType) -> Self {
        self.type_ = type_;
        self
    }

    /// 设置是否自动隐藏（默认 true）
    pub fn autohide(mut self, autohide: bool) -> Self {
        self.autohide = autohide;
        self
    }

    /// 设置触发元素（点击后推送通知）
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }
}

impl Styled for NotificationTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NotificationTrigger {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let title = self.title;
        let message = self.message;
        let type_ = self.type_;
        let autohide = self.autohide;

        let mut container = div()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                let mut note = Notification::new().with_type(type_).autohide(autohide);
                if let Some(t) = &title {
                    note = note.title(t.clone());
                }
                if let Some(m) = &message {
                    note = note.message(m.clone());
                }
                window.push_notification(note, cx);
                cx.stop_propagation();
            })
            .refine_style(&self.style);

        if let Some(trigger) = self.trigger {
            container = container.child(trigger);
        }

        container
    }
}

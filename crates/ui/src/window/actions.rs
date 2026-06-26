//! 窗口操作助手 —— 为 `&mut Window` 提供便捷的消息通知 API
//!
//! 自动通过 `WindowExt` 路由到 `Root` 管理的 `NotificationList`。
//! 在 ViewModel 中调用：`window.notify_info("已保存", cx);`

use gpui::{App, SharedString, Window};
use crate::{Notification, WindowExt};

/// 通知类型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

/// 窗口操作助手 trait
///
/// 为 `&mut Window` 提供便捷的消息通知 API。
/// 自动通过 `WindowExt` 路由到 `Root` 管理的 `NotificationList`。
pub trait IWindowActions {
    /// 显示一条通知（右下角，类似 VSCode）
    fn show_notification(
        &mut self,
        message: impl Into<SharedString>,
        kind: NotificationKind,
        cx: &mut App,
    );

    /// 显示信息通知
    fn notify_info(&mut self, message: impl Into<SharedString>, cx: &mut App) {
        self.show_notification(message, NotificationKind::Info, cx);
    }

    /// 显示成功通知
    fn notify_success(&mut self, message: impl Into<SharedString>, cx: &mut App) {
        self.show_notification(message, NotificationKind::Success, cx);
    }

    /// 显示警告通知
    fn notify_warning(&mut self, message: impl Into<SharedString>, cx: &mut App) {
        self.show_notification(message, NotificationKind::Warning, cx);
    }

    /// 显示错误通知
    fn notify_error(&mut self, message: impl Into<SharedString>, cx: &mut App) {
        self.show_notification(message, NotificationKind::Error, cx);
    }
}

impl IWindowActions for Window {
    fn show_notification(
        &mut self,
        message: impl Into<SharedString>,
        kind: NotificationKind,
        cx: &mut App,
    ) {
        let note = match kind {
            NotificationKind::Info => Notification::info(message),
            NotificationKind::Success => Notification::success(message),
            NotificationKind::Warning => Notification::warning(message),
            NotificationKind::Error => Notification::error(message),
        };
        self.push_notification(note, cx);
    }
}

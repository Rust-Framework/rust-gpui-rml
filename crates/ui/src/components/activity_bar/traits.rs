//! ActivityBar 相关 trait

use gpui::{AnyElement, App, SharedString, Window};
use gpui_component::IconName;

/// 活动栏面板项接口
pub trait IActivityPanel: Send + Sync + 'static {
    fn id(&self) -> SharedString;
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    /// 面板内容。`ActivityBar` 在渲染时调用当前激活面板的 `panel`。
    fn panel(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let _ = (window, cx);
        None
    }
}

/// 活动栏底部动作项接口
pub trait IActivityAct: Send + Sync + 'static {
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    fn on_click(&self, window: &mut Window, cx: &mut App);
}

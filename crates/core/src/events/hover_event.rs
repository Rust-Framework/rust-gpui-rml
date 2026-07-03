//! 悬停事件（onhover / onmouseenter / onmouseleave）
//!
//! GPUI 的 `on_hover` 回调接收 `&bool`（true = 进入，false = 离开），
//! RML 将其封装为 `HoverEvent`。

use crate::event::IEvent;

use super::flags::EventFlags;

/// 悬停事件（onhover / onmouseenter / onmouseleave）
#[derive(Debug, Clone, Default)]
pub struct HoverEvent {
    /// true 表示鼠标进入元素，false 表示离开
    pub is_hovering: bool,
    flags: EventFlags,
}

impl IEvent for HoverEvent {
    fn prevent_default(&mut self) {
        self.flags.prevent_default();
    }
    fn stop_propagation(&mut self) {
        self.flags.stop_propagation();
    }
    fn is_default_prevented(&self) -> bool {
        self.flags.is_default_prevented()
    }
    fn is_propagation_stopped(&self) -> bool {
        self.flags.is_propagation_stopped()
    }
}

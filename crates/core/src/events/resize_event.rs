//! 窗口大小变化事件（onresize）

use gpui::Pixels;

use crate::event::IEvent;

use super::flags::EventFlags;

/// 窗口大小变化事件（onresize）
#[derive(Debug, Clone, Default)]
pub struct ResizeEvent {
    pub width: Pixels,
    pub height: Pixels,
    flags: EventFlags,
}

impl IEvent for ResizeEvent {
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

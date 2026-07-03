//! 滚动事件（onscroll）

use gpui::Pixels;

use crate::event::IEvent;

use super::flags::EventFlags;

/// 滚动事件（onscroll）
#[derive(Debug, Clone, Default)]
pub struct ScrollEvent {
    pub scroll_x: Pixels,
    pub scroll_y: Pixels,
    flags: EventFlags,
}

impl IEvent for ScrollEvent {
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

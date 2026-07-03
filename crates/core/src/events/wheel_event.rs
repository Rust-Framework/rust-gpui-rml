//! 滚轮事件（onwheel）

use gpui::{Modifiers, Pixels, Point};

use crate::event::IEvent;

use super::flags::EventFlags;

/// 滚轮事件（onwheel）
#[derive(Debug, Clone, Default)]
pub struct WheelEvent {
    pub position: Point<Pixels>,
    pub delta_x: Pixels,
    pub delta_y: Pixels,
    pub modifiers: Modifiers,
    flags: EventFlags,
}

impl IEvent for WheelEvent {
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

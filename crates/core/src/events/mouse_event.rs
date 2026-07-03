//! 鼠标事件（onmousedown / onmouseup / onmouseenter / onmouseleave / onmousemove）

use gpui::{Modifiers, Pixels, Point};

use crate::event::IEvent;

use super::flags::EventFlags;
use super::mouse_button::MouseButton;

/// 鼠标事件（onmousedown / onmouseup / onmouseenter / onmouseleave / onmousemove）
#[derive(Debug, Clone, Default)]
pub struct MouseEvent {
    pub button: MouseButton,
    pub position: Point<Pixels>,
    pub modifiers: Modifiers,
    flags: EventFlags,
}

impl IEvent for MouseEvent {
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

//! 键盘按下事件（onkeydown）

use gpui::{Keystroke, Modifiers};

use crate::event::IEvent;

use super::flags::EventFlags;

/// 键盘按下事件（onkeydown）
#[derive(Debug, Clone)]
pub struct KeyDownEvent {
    pub key: Keystroke,
    pub modifiers: Modifiers,
    flags: EventFlags,
}

impl Default for KeyDownEvent {
    fn default() -> Self {
        Self {
            key: Keystroke::default(),
            modifiers: Modifiers::default(),
            flags: EventFlags::default(),
        }
    }
}

impl IEvent for KeyDownEvent {
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

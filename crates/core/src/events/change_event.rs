//! 变更事件（onchange）

use gpui::SharedString;

use crate::event::IEvent;

use super::flags::EventFlags;

/// 变更事件（onchange）
#[derive(Debug, Clone, Default)]
pub struct ChangeEvent {
    pub value: SharedString,
    flags: EventFlags,
}

impl IEvent for ChangeEvent {
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

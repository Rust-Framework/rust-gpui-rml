//! 加载完成事件（onload）

use crate::event::IEvent;

use super::flags::EventFlags;

/// 加载完成事件（onload）
#[derive(Debug, Clone, Default)]
pub struct LoadEvent {
    flags: EventFlags,
}

impl IEvent for LoadEvent {
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

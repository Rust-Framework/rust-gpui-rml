//! 焦点事件（onfocus / onblur）

use crate::event::IEvent;

use super::flags::EventFlags;

/// 焦点事件（onfocus / onblur）
#[derive(Debug, Clone, Default)]
pub struct FocusEvent {
    /// 获得或失去焦点的目标元素 ID（Phase A 为 None，Phase B 由运行时填充）
    pub target: Option<u64>,
    flags: EventFlags,
}

impl IEvent for FocusEvent {
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

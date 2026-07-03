//! 事件流控制标志位的公用辅助

/// 事件流控制标志位
#[derive(Debug, Clone, Default)]
pub(super) struct EventFlags {
    default_prevented: bool,
    propagation_stopped: bool,
}

impl EventFlags {
    pub(super) fn prevent_default(&mut self) {
        self.default_prevented = true;
    }
    pub(super) fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }
    pub(super) fn is_default_prevented(&self) -> bool {
        self.default_prevented
    }
    pub(super) fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
}

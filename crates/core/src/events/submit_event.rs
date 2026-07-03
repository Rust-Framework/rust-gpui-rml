//! 表单提交事件（onsubmit）

use gpui::SharedString;

use crate::event::IEvent;

use super::flags::EventFlags;

/// 表单提交事件（onsubmit）
#[derive(Debug, Clone, Default)]
pub struct SubmitEvent {
    /// 表单数据（Phase A 为空，Phase B 由运行时填充）
    pub form_data: std::collections::HashMap<SharedString, SharedString>,
    flags: EventFlags,
}

impl IEvent for SubmitEvent {
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

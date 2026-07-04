//! 输入事件（oninput）

use gpui::SharedString;

use crate::event::IEvent;

use super::flags::EventFlags;

/// 输入事件（oninput）
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct InputEvent {
    pub value: SharedString,
    /// 前一个值
    pub prev_value: SharedString,
    flags: EventFlags,
}


impl IEvent for InputEvent {
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

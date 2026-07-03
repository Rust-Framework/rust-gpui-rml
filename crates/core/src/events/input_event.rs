//! 输入事件（oninput）

use gpui::SharedString;

use crate::event::IEvent;

use super::flags::EventFlags;

/// 输入事件（oninput）
#[derive(Debug, Clone)]
pub struct InputEvent {
    pub value: SharedString,
    /// 前一个值（文档中写 `old_value`，保留 `prev_value` 作字段名，提供 `old_value()` 别名）
    pub prev_value: SharedString,
    flags: EventFlags,
}

impl InputEvent {
    /// 文档兼容别名
    pub fn old_value(&self) -> &SharedString {
        &self.prev_value
    }
}

impl Default for InputEvent {
    fn default() -> Self {
        Self {
            value: SharedString::default(),
            prev_value: SharedString::default(),
            flags: EventFlags::default(),
        }
    }
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

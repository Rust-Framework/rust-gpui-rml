//! 点击事件（onclick / ondblclick）

use gpui::{Modifiers, Pixels, Point};

use crate::event::IEvent;

use super::flags::EventFlags;
use super::mouse_button::MouseButton;

/// 点击事件（onclick / ondblclick）
#[derive(Debug, Clone, Default)]
pub struct ClickEvent {
    pub button: MouseButton,
    pub position: Point<Pixels>,
    /// 修饰键状态（文档 §5.2.2 要求）
    pub modifiers: Modifiers,
    /// 点击次数（区分单击/双击）
    pub click_count: u32,
    flags: EventFlags,
}

impl ClickEvent {
    /// 文档兼容别名：`old_value` / `prev_value` 同义
    pub fn new(position: Point<Pixels>) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }
}

impl IEvent for ClickEvent {
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

//! RML 事件对象类型
//!
//! RML 定义自己的事件对象，作为 GPUI 原生事件的抽象层。
//! 代码生成器负责将 GPUI 事件转换为 RML 事件后传递给命令方法。
//! 所有事件实现 `IEvent` trait，支持 `prevent_default` / `stop_propagation`。
//! 详见文档 §5.2 事件对象。

use crate::event::IEvent;
use gpui::{Keystroke, Modifiers, Pixels, Point, SharedString};

/// 鼠标按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
    Other(u16),
}

/// 事件流控制标志位的公用辅助
#[derive(Debug, Clone, Default)]
struct EventFlags {
    default_prevented: bool,
    propagation_stopped: bool,
}

impl EventFlags {
    fn prevent_default(&mut self) {
        self.default_prevented = true;
    }
    fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }
    fn is_default_prevented(&self) -> bool {
        self.default_prevented
    }
    fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
}

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

/// 滚轮事件（onwheel）
#[derive(Debug, Clone, Default)]
pub struct WheelEvent {
    pub position: Point<Pixels>,
    pub delta_x: Pixels,
    pub delta_y: Pixels,
    pub modifiers: Modifiers,
    flags: EventFlags,
}

impl IEvent for WheelEvent {
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

/// 键盘释放事件（onkeyup）
#[derive(Debug, Clone)]
pub struct KeyUpEvent {
    pub key: Keystroke,
    pub modifiers: Modifiers,
    flags: EventFlags,
}

impl Default for KeyUpEvent {
    fn default() -> Self {
        Self {
            key: Keystroke::default(),
            modifiers: Modifiers::default(),
            flags: EventFlags::default(),
        }
    }
}

impl IEvent for KeyUpEvent {
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

/// 窗口大小变化事件（onresize）
#[derive(Debug, Clone, Default)]
pub struct ResizeEvent {
    pub width: Pixels,
    pub height: Pixels,
    flags: EventFlags,
}

impl IEvent for ResizeEvent {
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

/// 滚动事件（onscroll）
#[derive(Debug, Clone, Default)]
pub struct ScrollEvent {
    pub scroll_x: Pixels,
    pub scroll_y: Pixels,
    flags: EventFlags,
}

impl IEvent for ScrollEvent {
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

/// 悬停事件（onhover / onmouseenter / onmouseleave）
///
/// GPUI 的 `on_hover` 回调接收 `&bool`（true = 进入，false = 离开），
/// RML 将其封装为 `HoverEvent`。
#[derive(Debug, Clone, Default)]
pub struct HoverEvent {
    /// true 表示鼠标进入元素，false 表示离开
    pub is_hovering: bool,
    flags: EventFlags,
}

impl IEvent for HoverEvent {
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

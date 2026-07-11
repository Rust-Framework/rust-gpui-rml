//! ColorPicker 组件封装 —— 基于 gpui-component 的 ColorPicker
//!
//! 颜色选择器，Stateful 构造器接受 &Entity<ColorPickerState>。
//! ColorPickerState::new(window, cx) 创建状态，ColorPicker::new(&state) 创建视图。
//!
//! ## 声明式语法
//!
//! ```rml
//! <ColorPicker ref="color_state" label="颜色" on-change={on_color_change} />
//! ```
//!
//! on-change 事件通过 ColorPickerEvent::Change(Option<Hsla>) 订阅，
//! 用户方法签名约定：`fn on_color_change(&mut self, color: Option<Hsla>, cx: &mut Context<Self>)`。

pub use gpui_component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};

//! DatePicker 组件封装 —— 基于 gpui-component 的 DatePicker
//!
//! 日期选择器，Stateful 构造器接受 &Entity<DatePickerState>。
//! DatePickerState::new(window, cx) 创建状态，DatePicker::new(&state) 创建视图。
//!
//! ## 声明式语法
//!
//! ```rml
//! <DatePicker ref="date_picker_state" placeholder="选择日期" cleanable on-change={on_date_change} />
//! ```
//!
//! on-change 事件通过 DatePickerEvent::Change(Date) 订阅，
//! 用户方法签名约定：`fn on_date_change(&mut self, date: Date, cx: &mut Context<Self>)`。

pub use gpui_component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};

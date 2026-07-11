//! Calendar 组件封装 —— 基于 gpui-component 的 Calendar
//!
//! 日历选择器，Stateful 构造器接受 &Entity<CalendarState>。
//! CalendarState::new(window, cx) 创建状态，Calendar::new(&state) 创建视图。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Calendar ref="calendar_state" on-select={on_date_select} />
//! ```
//!
//! on-select 事件通过 CalendarEvent::Selected(Date) 订阅，
//! 用户方法签名约定：`fn on_date_select(&mut self, date: Date, cx: &mut Context<Self>)`。

pub use gpui_component::calendar::{Calendar, CalendarEvent, CalendarState, Date};

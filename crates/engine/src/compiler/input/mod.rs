//! Input / TextInput 组件 codegen 模块入口。
//!
//! 构造器仍由 `component::gen_component` 的 `Stateful` 分支统一处理，
//! 本模块提供事件订阅代码生成（基于 `cx.subscribe` + `EventEmitter<InputEvent>`）。

pub mod event;

pub use event::{gen_input_event_subscribe, is_input_event};

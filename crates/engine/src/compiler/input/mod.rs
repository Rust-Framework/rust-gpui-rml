//! Input / TextInput 组件 codegen 模块入口。
//!
//! 构造器仍由 `component::gen_component` 的 `Stateful` 分支统一处理，
//! 本模块仅提供事件 setter。

pub mod event;

pub use event::event_setter;

//! Tree 组件 codegen 模块入口。
//!
//! Tree 是 StatefulWithDelegate 组件，构造逻辑由 `translator::component::tree` 处理。
//! 本模块仅保留 Tree 专用 event_setter（on_activate/on_select）。

pub mod setters;

pub use setters::event_setter;

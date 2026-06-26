//! RML 运行时支持
//!
//! Phase A：仅提供最小 stub，供 lib.rs 编译通过。
//! Phase B：实现事件流调度、组件注册表、样式系统、热重载 watcher。

pub mod event_flow;
pub mod component_registry;
pub mod styling;
pub mod watcher;

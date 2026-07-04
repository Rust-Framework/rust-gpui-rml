//! RML 运行时支持
//!
//! 当前仅提供事件流调度（`event_flow`）。组件注册表、样式系统、热重载 watcher
//! 等 Phase A stub 已移除（项目记忆约束：Phase C 被拒绝，不再新增宏与运行时机制）。

pub mod event_flow;

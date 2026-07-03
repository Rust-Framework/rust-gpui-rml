//! 调试会话管理
//!
//! 编排引擎 + 断点 + 调用栈 + 变量缓存 + 状态机。
//!
//! - `manager`：`DebugSession` 顶层编排器
//! - `breakpoints`：断点增删改查（纯数据）
//! - `callstack`：线程/栈帧缓存
//! - `variables`：变量树缓存（按 variablesReference）

pub mod breakpoints;
pub mod callstack;
pub mod manager;
pub mod variables;

pub use breakpoints::BreakpointManager;
pub use callstack::CallStack;
pub use manager::DebugSession;
pub use variables::VariableTree;

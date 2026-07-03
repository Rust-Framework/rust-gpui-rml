//! RML 调试适配器（DAP）
//!
//! 对标 `rust-rml-lsp` 的分层架构，为 RML + Rust 框架提供调试支持：
//!
//! | 层 | 模块 | 职责 | lsp 对应 |
//! |----|------|------|---------|
//! | 中性接口 | `engine` | `DebugEngine` trait + 中性类型 | `rust/query` |
//! | DAP 协议 | `protocol` | DAP 消息类型 + 帧编解码 | （lsp 用 lsp-server/lsp-types） |
//! | 会话管理 | `session` | 调试会话生命周期 + 断点/栈/变量 | `handlers`/`features` |
//! | 源映射 | `source_map` | `.rml` ↔ `.rml.rs` 位置映射 | `crosslang` |
//! | 引擎适配 | `lldb` | `LldbAdapter`（feature 门，隔离 lldb-dap 通信） | `rust/adapter` |
//!
//! 集成方式：spawn `lldb-dap` 子进程，通过 stdio 交流 DAP 协议；
//! `rust-lldb` 作为启动包装加载 Rust 符号源路径与 pretty-printer。

pub mod engine;
pub mod protocol;
pub mod session;
pub mod source_map;

#[cfg(feature = "lldb-backend")]
pub mod lldb;

pub use engine::{
    AttachConfig, Breakpoint, BreakpointResult, DebugEngine, DebugState, LaunchConfig, NoopEngine,
    Scope, StackFrame, StoppedReason, Thread, Variable,
};

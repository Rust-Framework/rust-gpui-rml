//! lldb-dap 引擎集成层
//!
//! 通过 `DebugEngine` trait 隔离 lldb-dap 子进程通信。
//! - `host.rs`：`LldbHost` 管理 `lldb-dap` 子进程生命周期
//! - `adapter.rs`：`LldbAdapter` 实现 `DebugEngine`（DAP 协议转换）
//!
//! 本模块仅在 `lldb-backend` feature 启用时编译，关闭时上层使用 `NoopEngine` 降级。
//! 对标 lsp crate 的 `rust/` 模块结构。

pub mod adapter;
pub mod host;

pub use adapter::LldbAdapter;
pub use host::LldbHost;

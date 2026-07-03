//! `lldb-dap` 子进程生命周期管理
//!
//! 对标 lsp crate 的 `rust/host.rs`（`RaHost` 管理 `AnalysisHost`+`Vfs`）。
//! 本模块管理 `lldb-dap` 子进程：spawn、stdin/stdout 句柄、kill。
//!
//! 集成方式（参照 VS Code）：
//! - spawn `lldb-dap`（LLVM 自带）或经 `rust-lldb` 包装启动（加载 Rust 符号源路径
//!   与 pretty-printer）
//! - 通过 stdin 发送 DAP 请求，从 stdout 读取 DAP 响应/事件
//! - stderr 用于诊断日志
//!
//! 本期为骨架：类型定义 + 生命周期占位，实际 spawn 与 stdio 异步读写循环待实现。

use std::sync::Mutex;

use anyhow::Result;

/// lldb-dap 后端句柄：持有子进程与 stdio 句柄
///
/// `spawn` 耗时较短（仅启动子进程），但 DAP `initialize`/`launch` 握手需往返通信。
/// 加载完成前 `is_started()` 返回 false。
pub struct LldbHost {
    inner: Mutex<LldbHostInner>,
}

struct LldbHostInner {
    /// 子进程句柄（spawn 后存在，disconnect 后 None）
    #[allow(dead_code)]
    child: Option<tokio::process::Child>,
    /// 是否已成功握手（DAP `initialize` 完成）
    started: bool,
}

impl LldbHost {
    /// 创建未启动的后端
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(LldbHostInner {
                child: None,
                started: false,
            }),
        }
    }

    /// 启动 `lldb-dap` 子进程并完成 DAP 握手
    ///
    /// `rust_lldb_wrapper`：为 Some 时经 `rust-lldb` 包装启动（加载 Rust 符号路径），
    /// 为 None 时直接 spawn `lldb-dap`。
    pub fn spawn(&self, _rust_lldb_wrapper: Option<&str>) -> Result<()> {
        // TODO: spawn lldb-dap（或 rust-lldb 包装），建立 stdin/stdout/stderr 句柄，
        //       发送 DAP `initialize` 请求并等待 `initialized` 事件。
        todo!("lldb-dap subprocess spawn + DAP handshake not yet implemented")
    }

    /// 是否已启动（子进程存活且握手完成）
    pub fn is_started(&self) -> bool {
        self.inner.lock().map(|i| i.started).unwrap_or(false)
    }

    /// 标记握手完成（由 adapter 在收到 `initialized` 事件后调用）
    pub fn mark_started(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.started = true;
        }
    }

    /// 终止子进程
    pub fn kill(&self) -> Result<()> {
        // TODO: 向 lldb-dap 发送 DAP `disconnect` 请求，等待子进程退出，必要时 kill。
        todo!("lldb-dap subprocess termination not yet implemented")
    }
}

impl Default for LldbHost {
    fn default() -> Self {
        Self::new()
    }
}

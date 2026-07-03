//! `DebugEngine` 的 lldb-dap 实现
//!
//! 对标 lsp crate 的 `rust/adapter.rs`（`RaAdapter` 实现 `RustSemanticQuery`）。
//! 本文件是隔离层的关键：所有 lldb-dap / DAP 协议交互在此完成，
//! 上层（session / source_map）只依赖 `DebugEngine` 中性接口。
//!
//! 本期为骨架：类型定义 + trait 实现 stub（`todo!()`），可编译通过。
//! 完整 DAP 请求/响应/事件交互实现留待后续迭代。

use std::sync::Arc;

use super::host::LldbHost;
use crate::engine::{
    AttachConfig, Breakpoint, BreakpointResult, DebugEngine, LaunchConfig, Scope, StackFrame,
    Thread, Variable,
};

/// lldb-dap 适配器：桥接 `DebugEngine` 到 `lldb-dap` 子进程
pub struct LldbAdapter {
    host: Arc<LldbHost>,
}

impl LldbAdapter {
    pub fn new(host: Arc<LldbHost>) -> Self {
        Self { host }
    }
}

impl DebugEngine for LldbAdapter {
    fn launch(&mut self, _config: &LaunchConfig) -> anyhow::Result<()> {
        // TODO: DAP `launch` 请求（program/args/cwd/env/stopOnEntry），
        //       翻译 LaunchConfig → DAP arguments，发送并等待响应。
        todo!("DAP launch request not yet implemented")
    }

    fn attach(&mut self, _config: &AttachConfig) -> anyhow::Result<()> {
        // TODO: DAP `attach` 请求（pid/program/stopOnAttach）。
        todo!("DAP attach request not yet implemented")
    }

    fn configuration_done(&mut self) -> anyhow::Result<()> {
        // TODO: DAP `configurationDone` 请求，通知 lldb-dap 断点已设置完毕。
        todo!("DAP configurationDone request not yet implemented")
    }

    fn disconnect(&mut self) -> anyhow::Result<()> {
        // TODO: DAP `disconnect` 请求 + 终止子进程。
        self.host.kill()
    }

    fn is_started(&self) -> bool {
        self.host.is_started()
    }

    fn set_breakpoints(
        &mut self,
        _breakpoints: &[Breakpoint],
    ) -> anyhow::Result<Vec<BreakpointResult>> {
        // TODO: DAP `setBreakpoints` 请求，按 source 分组，翻译 Breakpoint → DAP source/breakpoints，
        //       解析响应中的 verified/line/message → BreakpointResult。
        todo!("DAP setBreakpoints request not yet implemented")
    }

    fn continue_(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        // TODO: DAP `continue` 请求。
        todo!("DAP continue request not yet implemented")
    }

    fn step_over(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        // TODO: DAP `next` 请求。
        todo!("DAP next request not yet implemented")
    }

    fn step_in(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        // TODO: DAP `stepIn` 请求。
        todo!("DAP stepIn request not yet implemented")
    }

    fn step_out(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        // TODO: DAP `stepOut` 请求。
        todo!("DAP stepOut request not yet implemented")
    }

    fn pause(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        // TODO: DAP `pause` 请求。
        todo!("DAP pause request not yet implemented")
    }

    fn threads(&self) -> anyhow::Result<Vec<Thread>> {
        // TODO: DAP `threads` 请求，解析响应 → Vec<Thread>。
        todo!("DAP threads request not yet implemented")
    }

    fn stack_trace(&self, _thread_id: u64) -> anyhow::Result<Vec<StackFrame>> {
        // TODO: DAP `stackTrace` 请求，解析响应 → Vec<StackFrame>。
        todo!("DAP stackTrace request not yet implemented")
    }

    fn scopes(&self, _frame_id: u64) -> anyhow::Result<Vec<Scope>> {
        // TODO: DAP `scopes` 请求，解析响应 → Vec<Scope>。
        todo!("DAP scopes request not yet implemented")
    }

    fn variables(&self, _variables_reference: u64) -> anyhow::Result<Vec<Variable>> {
        // TODO: DAP `variables` 请求，解析响应 → Vec<Variable>。
        todo!("DAP variables request not yet implemented")
    }

    fn evaluate(&self, _expression: &str, _frame_id: u64) -> anyhow::Result<Option<String>> {
        // TODO: DAP `evaluate` 请求，解析响应 → String。
        todo!("DAP evaluate request not yet implemented")
    }
}

//! 调试会话编排
//!
//! `DebugSession` 持有引擎 + 断点 + 调用栈 + 变量缓存 + 状态机，编排各模块协作。
//! 泛型 `E: DebugEngine` 使其可工作于 `NoopEngine`（降级）或 `LldbAdapter`（lldb-backend）。
//!
//! 状态流转：`Unstarted` → `launch`/`attach` → `Running` ↔ `Paused` → `Terminated` → `disconnect` → `Unstarted`

use lsp_types::Url;

use crate::engine::{
    AttachConfig, Breakpoint, BreakpointResult, DebugEngine, DebugState, LaunchConfig, Scope,
    Variable,
};
use crate::session::breakpoints::BreakpointManager;
use crate::session::callstack::CallStack;
use crate::session::variables::VariableTree;

/// 调试会话：编排引擎与各状态模块
pub struct DebugSession<E: DebugEngine> {
    engine: E,
    breakpoints: BreakpointManager,
    callstack: CallStack,
    variables: VariableTree,
    state: DebugState,
    selected_thread: Option<u64>,
    selected_frame: Option<u64>,
}

impl<E: DebugEngine> DebugSession<E> {
    pub fn new(engine: E) -> Self {
        Self {
            engine,
            breakpoints: BreakpointManager::new(),
            callstack: CallStack::new(),
            variables: VariableTree::new(),
            state: DebugState::Unstarted,
            selected_thread: None,
            selected_frame: None,
        }
    }

    /// 当前会话状态
    pub fn state(&self) -> DebugState {
        self.state
    }

    // ── 会话生命周期 ──

    /// 启动被调试程序并提交已注册断点
    pub fn launch(&mut self, config: &LaunchConfig) -> anyhow::Result<()> {
        self.engine.launch(config)?;
        self.engine.configuration_done()?;
        self.state = DebugState::Running;
        self.clear_runtime_caches();
        Ok(())
    }

    /// 附加到已运行进程
    pub fn attach(&mut self, config: &AttachConfig) -> anyhow::Result<()> {
        self.engine.attach(config)?;
        self.engine.configuration_done()?;
        self.state = if config.stop_on_attach {
            DebugState::Paused
        } else {
            DebugState::Running
        };
        self.clear_runtime_caches();
        Ok(())
    }

    /// 断开会话
    pub fn disconnect(&mut self) -> anyhow::Result<()> {
        self.engine.disconnect()?;
        self.state = DebugState::Unstarted;
        self.clear_runtime_caches();
        self.selected_thread = None;
        self.selected_frame = None;
        Ok(())
    }

    // ── 断点（同步到引擎）──

    /// 替换指定源文件的断点列表并同步到引擎
    pub fn set_breakpoints(
        &mut self,
        source: &Url,
        breakpoints: Vec<Breakpoint>,
    ) -> anyhow::Result<Vec<BreakpointResult>> {
        let results = self.engine.set_breakpoints(&breakpoints)?;
        self.breakpoints.set(source, breakpoints);
        Ok(results)
    }

    /// 追加单个断点并同步整个文件到引擎
    pub fn add_breakpoint(&mut self, bp: Breakpoint) -> anyhow::Result<Vec<BreakpointResult>> {
        let source = bp.source.clone();
        self.breakpoints.add(bp);
        let all: Vec<Breakpoint> = self.breakpoints.get(&source).to_vec();
        let results = self.engine.set_breakpoints(&all)?;
        Ok(results)
    }

    /// 移除断点并同步
    pub fn remove_breakpoint(
        &mut self,
        source: &Url,
        line: u32,
    ) -> anyhow::Result<Vec<BreakpointResult>> {
        self.breakpoints.remove(source, line);
        let all: Vec<Breakpoint> = self.breakpoints.get(source).to_vec();
        let results = self.engine.set_breakpoints(&all)?;
        Ok(results)
    }

    /// 切换断点启用状态并同步
    pub fn toggle_breakpoint(
        &mut self,
        source: &Url,
        line: u32,
    ) -> anyhow::Result<Option<Vec<BreakpointResult>>> {
        match self.breakpoints.toggle(source, line) {
            Some(_) => {
                let all: Vec<Breakpoint> = self.breakpoints.get(source).to_vec();
                let results = self.engine.set_breakpoints(&all)?;
                Ok(Some(results))
            }
            None => Ok(None),
        }
    }

    // ── 执行控制（更新状态机）──

    pub fn continue_(&mut self, thread_id: u64) -> anyhow::Result<()> {
        self.engine.continue_(thread_id)?;
        self.state = DebugState::Running;
        self.selected_thread = Some(thread_id);
        self.clear_runtime_caches();
        Ok(())
    }

    pub fn step_over(&mut self, thread_id: u64) -> anyhow::Result<()> {
        self.engine.step_over(thread_id)?;
        self.state = DebugState::Running;
        self.clear_runtime_caches();
        Ok(())
    }

    pub fn step_in(&mut self, thread_id: u64) -> anyhow::Result<()> {
        self.engine.step_in(thread_id)?;
        self.state = DebugState::Running;
        self.clear_runtime_caches();
        Ok(())
    }

    pub fn step_out(&mut self, thread_id: u64) -> anyhow::Result<()> {
        self.engine.step_out(thread_id)?;
        self.state = DebugState::Running;
        self.clear_runtime_caches();
        Ok(())
    }

    pub fn pause(&mut self, thread_id: u64) -> anyhow::Result<()> {
        self.engine.pause(thread_id)?;
        Ok(())
    }

    // ── 事件钩子（由传输层在收到 DAP 事件时回调）──

    /// 引擎报告已暂停
    pub fn on_stopped(&mut self, thread_id: u64) -> anyhow::Result<()> {
        self.state = DebugState::Paused;
        self.selected_thread = Some(thread_id);
        self.refresh_threads()?;
        self.refresh_frames(thread_id)?;
        Ok(())
    }

    /// 引擎报告已终止
    pub fn on_terminated(&mut self) {
        self.state = DebugState::Terminated;
        self.clear_runtime_caches();
        self.selected_thread = None;
        self.selected_frame = None;
    }

    // ── 状态查询（委托引擎 + 缓存）──

    pub fn threads(&self) -> anyhow::Result<Vec<crate::engine::Thread>> {
        self.engine.threads()
    }

    pub fn stack_trace(&self, thread_id: u64) -> anyhow::Result<Vec<crate::engine::StackFrame>> {
        self.engine.stack_trace(thread_id)
    }

    /// 获取栈帧作用域并选中该帧
    pub fn scopes(&mut self, frame_id: u64) -> anyhow::Result<Vec<Scope>> {
        self.selected_frame = Some(frame_id);
        self.engine.scopes(frame_id)
    }

    /// 获取变量并缓存
    pub fn variables(&mut self, variables_reference: u64) -> anyhow::Result<Vec<Variable>> {
        let vars = self.engine.variables(variables_reference)?;
        self.variables.set(variables_reference, vars.clone());
        Ok(vars)
    }

    pub fn evaluate(&self, expression: &str, frame_id: u64) -> anyhow::Result<Option<String>> {
        self.engine.evaluate(expression, frame_id)
    }

    // ── 内部 ──

    fn refresh_threads(&mut self) -> anyhow::Result<()> {
        let threads = self.engine.threads()?;
        self.callstack.set_threads(threads);
        Ok(())
    }

    fn refresh_frames(&mut self, thread_id: u64) -> anyhow::Result<()> {
        let frames = self.engine.stack_trace(thread_id)?;
        self.callstack.set_frames(thread_id, frames);
        Ok(())
    }

    fn clear_runtime_caches(&mut self) {
        self.callstack.clear();
        self.variables.clear();
    }
}

//! 调试引擎隔离层
//!
//! 定义 `DebugEngine` trait + 中性类型，封装所有调试器后端交互。
//! 对标 `rust-rml-lsp` 的 `RustSemanticQuery`：上层（session / source_map）只依赖本 trait，
//! 不接触任何 lldb-dap / DAP 协议细节。lldb 后端升级或替换时只需修改 `lldb/adapter.rs`，
//! 其余代码零改动。
//!
//! ## 设计原则
//!
//! - trait 接口零引擎依赖：方法签名只用 `Url`/`u32` 等中性类型
//! - `NoopEngine` 始终可用（非 feature 门）：lldb-backend 关闭或未启动时降级返回空
//! - 行号统一 1-based（DAP 惯例），适配器负责与具体后端的基偏移转换

use std::path::PathBuf;

use lsp_types::Url;

// ──────────────────────────────────────────────────────────────────────────
// 中性类型（无 lldb 依赖）
// ──────────────────────────────────────────────────────────────────────────

/// 调试会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugState {
    /// 未启动（未调用 launch/attach，或已 disconnect）
    Unstarted,
    /// 运行中（已 launch，未暂停）
    Running,
    /// 已暂停（命中断点 / 主动暂停 / 单步停留）
    Paused,
    /// 已终止（被调试进程退出）
    Terminated,
}

/// 断点（用户意图）
///
/// 描述用户在编辑器中设置的断点，`line` 为 1-based。
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub source: Url,
    pub line: u32,
    /// 条件表达式（为 None 表示无条件断点）
    pub condition: Option<String>,
    /// 命中次数条件（如 ">=5"，为 None 表示总是命中）
    pub hit_condition: Option<String>,
    /// 日志点消息（为 None 表示普通断点而非日志点）
    pub log_message: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 断点在引擎中的校验结果
///
/// 引擎收到断点请求后返回：是否成功绑定 + 实际命中位置（可能与请求位置不同）。
#[derive(Debug, Clone)]
pub struct BreakpointResult {
    pub verified: bool,
    /// 引擎实际绑定的位置（与请求可能不同，如优化代码行偏移）
    pub actual_line: Option<u32>,
    pub message: Option<String>,
}

/// 调试线程
#[derive(Debug, Clone)]
pub struct Thread {
    pub id: u64,
    pub name: String,
}

/// 调用栈帧
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub id: u64,
    pub name: String,
    pub source: Option<Url>,
    pub line: u32,
    pub column: u32,
    /// 模块/库标识（如 "my_app" 或 "std::core"）
    pub module: Option<String>,
}

/// 变量作用域（如 Local / Arguments / Registers）
#[derive(Debug, Clone)]
pub struct Scope {
    pub name: String,
    /// 引用此值可取该作用域下的变量列表（DAP variablesReference）
    pub variables_reference: u64,
    pub expensive: bool,
}

/// 变量值
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_str: Option<String>,
    /// 引用此值可取嵌套变量（如 struct 字段、集合元素）
    pub variables_reference: u64,
}

/// 暂停原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoppedReason {
    Breakpoint,
    Step,
    Exception,
    Entry,
    Pause,
}

/// 启动配置
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    /// 被调试程序路径
    pub program: PathBuf,
    /// 命令行参数
    pub args: Vec<String>,
    /// 工作目录
    pub cwd: Option<PathBuf>,
    /// 环境变量（追加到当前环境）
    pub env: Vec<(String, String)>,
    /// 启动后是否立即暂停（false = 直接到 main 入口断点）
    pub stop_on_entry: bool,
}

/// 附加配置（附加到已运行进程）
#[derive(Debug, Clone)]
pub struct AttachConfig {
    pub pid: u32,
    pub program: Option<PathBuf>,
    pub stop_on_attach: bool,
}

// ──────────────────────────────────────────────────────────────────────────
// 隔离 trait
// ──────────────────────────────────────────────────────────────────────────

/// 调试引擎抽象
///
/// 所有 lldb-dap / DAP 协议细节（子进程管理、消息帧编解码、序列号管理）
/// 绝不出现在 trait 接口中。实现方（`LldbAdapter`）负责协议转换与异步通信。
///
/// 生命周期：`launch`/`attach` → `configuration_done` → 运行 →
/// （`set_breakpoints` 等可随时调用）→ `continue_`/`step_*` → `disconnect`
pub trait DebugEngine: Send + Sync {
    // ── 会话生命周期 ──

    /// 启动被调试程序
    fn launch(&mut self, config: &LaunchConfig) -> anyhow::Result<()>;

    /// 附加到已运行进程
    fn attach(&mut self, config: &AttachConfig) -> anyhow::Result<()>;

    /// 通知引擎配置完成（断点已设置），可开始执行
    fn configuration_done(&mut self) -> anyhow::Result<()>;

    /// 断开会话（终止被调试进程）
    fn disconnect(&mut self) -> anyhow::Result<()>;

    /// 引擎是否已启动（launch/attach 成功后为 true，disconnect 后为 false）
    fn is_started(&self) -> bool;

    // ── 断点 ──

    /// 为指定源文件设置断点列表（替换该文件已有断点）
    fn set_breakpoints(
        &mut self,
        breakpoints: &[Breakpoint],
    ) -> anyhow::Result<Vec<BreakpointResult>>;

    // ── 执行控制 ──

    /// 继续执行
    fn continue_(&mut self, thread_id: u64) -> anyhow::Result<()>;

    /// 单步跳过（next）
    fn step_over(&mut self, thread_id: u64) -> anyhow::Result<()>;

    /// 单步进入（step in）
    fn step_in(&mut self, thread_id: u64) -> anyhow::Result<()>;

    /// 单步跳出（step out）
    fn step_out(&mut self, thread_id: u64) -> anyhow::Result<()>;

    /// 暂停执行
    fn pause(&mut self, thread_id: u64) -> anyhow::Result<()>;

    // ── 状态查询 ──

    /// 列出所有线程
    fn threads(&self) -> anyhow::Result<Vec<Thread>>;

    /// 获取指定线程的调用栈
    fn stack_trace(&self, thread_id: u64) -> anyhow::Result<Vec<StackFrame>>;

    /// 获取指定栈帧的变量作用域
    fn scopes(&self, frame_id: u64) -> anyhow::Result<Vec<Scope>>;

    /// 获取指定 variablesReference 下的变量
    fn variables(&self, variables_reference: u64) -> anyhow::Result<Vec<Variable>>;

    /// 在指定栈帧上下文求值表达式
    fn evaluate(&self, expression: &str, frame_id: u64) -> anyhow::Result<Option<String>>;
}

// ──────────────────────────────────────────────────────────────────────────
// 降级实现（lldb-backend feature 关闭或引擎未启动时使用）
// ──────────────────────────────────────────────────────────────────────────

/// 空实现：lldb-dap 不可用或会话未启动时使用
///
/// 对标 `rust-rml-lsp` 的 `NoopQuery`：所有方法返回未启动错误或空集合，
/// 保证上层代码在无调试后端时仍可编译运行。
pub struct NoopEngine;

impl DebugEngine for NoopEngine {
    fn launch(&mut self, _config: &LaunchConfig) -> anyhow::Result<()> {
        anyhow::bail!("debug engine not available (lldb-backend feature disabled)")
    }

    fn attach(&mut self, _config: &AttachConfig) -> anyhow::Result<()> {
        anyhow::bail!("debug engine not available (lldb-backend feature disabled)")
    }

    fn configuration_done(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn disconnect(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_started(&self) -> bool {
        false
    }

    fn set_breakpoints(
        &mut self,
        _breakpoints: &[Breakpoint],
    ) -> anyhow::Result<Vec<BreakpointResult>> {
        Ok(Vec::new())
    }

    fn continue_(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        anyhow::bail!("debug engine not started")
    }

    fn step_over(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        anyhow::bail!("debug engine not started")
    }

    fn step_in(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        anyhow::bail!("debug engine not started")
    }

    fn step_out(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        anyhow::bail!("debug engine not started")
    }

    fn pause(&mut self, _thread_id: u64) -> anyhow::Result<()> {
        anyhow::bail!("debug engine not started")
    }

    fn threads(&self) -> anyhow::Result<Vec<Thread>> {
        Ok(Vec::new())
    }

    fn stack_trace(&self, _thread_id: u64) -> anyhow::Result<Vec<StackFrame>> {
        Ok(Vec::new())
    }

    fn scopes(&self, _frame_id: u64) -> anyhow::Result<Vec<Scope>> {
        Ok(Vec::new())
    }

    fn variables(&self, _variables_reference: u64) -> anyhow::Result<Vec<Variable>> {
        Ok(Vec::new())
    }

    fn evaluate(&self, _expression: &str, _frame_id: u64) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}

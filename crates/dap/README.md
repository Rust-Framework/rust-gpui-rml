# rust-rml-dap

> RML 调试适配器 —— lldb-dap 引擎集成 + DAP 协议客户端 + 调试会话管理 + RML 源映射。

## 职责

为 RML + Rust 框架提供专职调试支持，对标 `rust-rml-lsp` 的分层架构：

- **调试引擎集成**：通过子进程 `lldb-dap`（LLVM 自带）作为调试引擎，stdio 交流 DAP 协议；
  `rust-lldb`（Rust 工具链自带的 LLDB 包装）可作为启动包装加载 Rust 符号源路径与 pretty-printer
- **DAP 协议客户端**：实现 DAP（Debug Adapter Protocol）消息编解码与请求/响应/事件往返，
  隔离 lldb-dap 协议细节
- **调试会话管理**：编排启动/附加/断点/执行控制/状态查询，维护状态机
  （`Unstarted → Running ↔ Paused → Terminated`）
- **RML 源映射**：桥接 `.rml` 声明层与生成的 `.rml.rs` 代码层，将用户在 `.rml` 中设置的
  断点翻译为 `.rml.rs` 位置供引擎使用，将引擎停止位置翻译回 `.rml` 供 UI 高亮

## 非职责

- **调试 UI 组件**（断点栏、变量树、调用栈面板、Watch 面板）→ 归 `rust-rml-ui`
- **编译器/语法/代码生成** → 归 `rust-rml-engine`
- **`.rml` 行号 → 生成 `.rs` 行号的精确源映射** → 依赖 engine codegen 增强（另立任务），
  本 crate 用 `SourceMapper` trait 抽象占位，MVP 仅做文件级配对

## 架构总览

对标 `rust-rml-lsp` 的「中性 trait + feature 门 + 降级实现」模式：

| 层 | 模块 | 职责 | lsp 对应 |
|----|------|------|---------|
| 中性接口 | `engine` | `DebugEngine` trait + 中性调试类型（零 lldb 依赖） | `rust/query` |
| DAP 协议 | `protocol` | DAP 消息信封 + `Content-Length` 帧编解码 | `lsp-server`/`lsp-types` |
| 会话管理 | `session` | 调试会话生命周期 + 断点/栈/变量状态机 | `handlers`/`features` |
| 源映射 | `source_map` | `.rml` ↔ `.rml.rs` 位置双向翻译 | `crosslang` |
| 引擎适配 | `lldb` | `LldbAdapter` 实现 `DebugEngine`，隔离 lldb-dap 通信 | `rust/adapter` |

关键设计原则：
- **trait 接口零引擎依赖**：`DebugEngine` 方法签名只用 `Url`/`u32` 等中性类型，
  lldb-dap / DAP 协议细节绝不出现在 trait 接口中
- **feature 门隔离**：`lldb-backend` feature 控制 lldb 适配器与 `tokio` 依赖，
  关闭时 `NoopEngine` 降级（对标 lsp 的 `rust-backend`）
- **宿主生命周期分离**：`LldbHost` 只管子进程 spawn/kill 与握手状态，
  `LldbAdapter` 只做 DAP 协议转换

## 集成方式

```
┌─────────────────────────────────────────────────────────┐
│  rust-rml-ui（调试 UI 组件）                              │
│  断点栏 / 变量树 / 调用栈面板                             │
└───────────────────────┬─────────────────────────────────┘
                        │  DebugSession API（状态查询 + 执行控制）
┌───────────────────────▼─────────────────────────────────┐
│  rust-rml-dap（本 crate）                                │
│  ┌─────────────┐  ┌──────────┐  ┌────────────────────┐  │
│  │ DebugEngine │  │ session  │  │ source_map         │  │
│  │  trait      │◄─│ manager  │  │ .rml ↔ .rml.rs     │  │
│  └──────┬──────┘  └──────────┘  └────────────────────┘  │
│         │ LldbAdapter（feature 门）                       │
│         │ DAP 协议（protocol 模块）                       │
└─────────┼───────────────────────────────────────────────┘
          │ stdio（DAP 消息帧）
   ┌──────▼──────┐
   │  lldb-dap   │  ← LLVM 自带 DAP 适配器（或经 rust-lldb 包装）
   │  子进程     │
   └─────────────┘
```

## 模块结构

| 模块 | 核心类型 | 职责 |
|------|---------|------|
| `engine` | `DebugEngine`/`NoopEngine` | 调试引擎抽象 trait + 中性类型（`Breakpoint`/`StackFrame`/`Variable` 等） |
| `protocol/types` | `Message`/`Request`/`Response`/`Event` | DAP 消息信封（JSON-RPC 变体） |
| `protocol/codec` | `encode_message`/`decode_message` | `Content-Length` 帧编解码（纯函数） |
| `session/manager` | `DebugSession<E>` | 会话顶层编排：状态机 + 引擎委托 + 缓存同步 |
| `session/breakpoints` | `BreakpointManager` | 断点增删改查（纯数据，按源文件组织） |
| `session/callstack` | `CallStack` | 线程/栈帧缓存 |
| `session/variables` | `VariableTree` | 变量树缓存（按 `variablesReference`） |
| `source_map/mapper` | `SourceMapper`/`FilePairMapper` | 源映射 trait + 文件级配对 MVP |
| `lldb/host` | `LldbHost` | `lldb-dap` 子进程生命周期（feature 门） |
| `lldb/adapter` | `LldbAdapter` | `DebugEngine` 的 lldb-dap 实现（feature 门） |

## Feature 门

`lldb-backend`（默认关闭，对标 lsp 的 `rust-backend`）：

- **关闭时**：`lldb` 模块不编译，无 `tokio` 依赖，`NoopEngine` 提供降级（所有方法返回未启动/空）
- **启用时**：`lldb/host` + `lldb/adapter` 编译，引入 `tokio`（process/io-util/rt/sync）用于子进程 stdio 异步通信

```toml
[dependencies]
rust-rml-dap = { workspace = true, features = ["lldb-backend"] }
```

## RML 源映射

MVP 采用文件级配对（`FilePairMapper`）：

- 注册 `.rml` ↔ `.rml.rs` 文件对
- 正向映射：`.rml` 断点 → 配对的 `.rml.rs` 文件（行号原样传递，不翻译）
- 反向映射：`.rml.rs` 停止位置 → 配对的 `.rml` 文件

**精确行级映射**（如 `.rml` 第 5 行 → 生成的 `.rml.rs` 第 42 行）需 `rust-rml-engine` 的
codegen 增强输出 `.rml.map` 源映射文件，属另一任务。届时只需实现 `LineAccurateMapper`
满足 `SourceMapper` trait，上层代码无需改动。

## 用法

### 引入依赖

```toml
# Cargo.toml（应用 crate）
[dependencies]
rust-rml-dap = { workspace = true }                    # 降级模式（无 lldb）
# 或
rust-rml-dap = { workspace = true, features = ["lldb-backend"] }  # 完整模式
```

### 启动调试会话（概念示例）

```rust
use rml_dap::engine::{DebugEngine, LaunchConfig, NoopEngine};
use rml_dap::session::DebugSession;
use std::path::PathBuf;

// 降级模式：NoopEngine（lldb-backend 关闭时）
let mut session = DebugSession::new(NoopEngine);

let config = LaunchConfig {
    program: PathBuf::from("target/debug/my-app"),
    args: vec![],
    cwd: None,
    env: vec![],
    stop_on_entry: false,
};

// 完整模式（lldb-backend 启用时）：
// use rml_dap::lldb::{LldbAdapter, LldbHost};
// use std::sync::Arc;
// let host = Arc::new(LldbHost::new());
// host.spawn(None).expect("failed to start lldb-dap");
// let mut session = DebugSession::new(LldbAdapter::new(host));

session.launch(&config).expect("launch failed");
```

### 设置断点（经 RML 源映射翻译）

```rust
use rml_dap::engine::Breakpoint;
use rml_dap::source_map::{FilePairMapper, SourceMapper};
use lsp_types::Url;

let mut mapper = FilePairMapper::new();
mapper.register_pair(
    Url::parse("file:///src/login.rml").unwrap(),
    Url::parse("file:///src/login.rml.rs").unwrap(),
);

// 用户在 .rml 第 10 行第 3 列设断点 → 翻译为 .rml.rs 位置
let (rust_uri, line, column) = mapper.rml_to_rust(
    &Url::parse("file:///src/login.rml").unwrap(),
    10,
    3,
).unwrap();

let bp = Breakpoint {
    source: rust_uri,
    line,
    condition: None,
    hit_condition: None,
    log_message: None,
    enabled: true,
};
session.add_breakpoint(bp).expect("set breakpoint failed");
```

## 未来工作

- **DAP 协议完整实现**：`lldb/adapter` 内所有请求/响应/事件交互（launch/setBreakpoints/
  stackTrace/variables/evaluate 等）
- **lldb-dap 子进程 stdio 异步读写循环**：`lldb/host` 内基于 `tokio` 的事件泵
- **RML 精确源映射**：待 engine codegen 输出 `.rml.map`，实现 `LineAccurateMapper`
- **调试 UI 组件**：断点栏、变量树、调用栈面板、Watch 面板 → `rust-rml-ui`
- **demo 集成**：在 demo app 中演示完整调试流程

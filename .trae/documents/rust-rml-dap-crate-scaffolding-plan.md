# rust-rml-dap Crate 脚手架与设计计划

## Summary

在 `crates/dap` 下新建 `rust-rml-dap` 包，作为 RML + Rust 框架的调试支持专职 crate。以
`rust-lldb`（Rust 工具链自带的 LLDB 包装）作为调试引擎，通过 **子进程 `lldb-dap` + DAP
协议** 集成（与 VS Code 等编辑器同款标准方案）。

crate 职责范围（用户已确认）：**引擎适配 + DAP 客户端 + 会话管理 + RML 源映射**，对标
`rust-rml-lsp` 的完整职责分层；调试 UI 组件留给 `rust-rml-ui`，不在此 crate 内。

本次交付为 **crate 脚手架 + 架构设计 + README**：定义核心 trait/类型契约，模块骨架可编译，
具体 DAP 协议交互与 lldb-dap 通信的完整实现留待后续迭代。

## Current State Analysis

### 现有架构参考（`rust-rml-lsp`）

`crates/lsp` 是本次设计的直接模板，其分层模式：

| 层 | 文件 | 职责 | 本 crate 对应 |
|----|------|------|--------------|
| 中性接口 | `rust/query.rs` | `RustSemanticQuery` trait + 中性类型，无 `ra_ap_*` 依赖 | `engine.rs` |
| 引擎宿主 | `rust/host.rs` | `RaHost`：`AnalysisHost`+`Vfs` 生命周期，`#[cfg(feature="rust-backend")]` | `lldb/host.rs` |
| 引擎适配 | `rust/adapter.rs` | `RaAdapter` 实现 trait，隔离所有 `ra_ap_*` 转换 | `lldb/adapter.rs` |
| 降级实现 | `rust/mod.rs` | `NoopQuery`（feature 关闭时） | `lldb/mod.rs` |
| feature 门 | `Cargo.toml` | `rust-backend` feature 控制 `ra_ap_*` 依赖 | `lldb-backend` feature |

关键设计原则（已验证）：
- **trait 接口零引擎依赖**：`RustSemanticQuery` 方法签名只用 `Url`/`Position` 等中性类型
- **feature 门隔离**：`ra_ap_*` 依赖在 `rust-backend` feature 后，关闭时 `NoopQuery` 降级
- **宿主生命周期分离**：`RaHost` 只管加载/就绪状态，`RaAdapter` 只做查询转换

### RML 源映射现状

**关键约束**：`crates/engine/src/build/` 的 codegen 当前 **不维护** `.rml` 行号 → 生成 `.rs`
行号的源映射。`build/cache.rs` 的 `Cache` 结构只存 sha256 哈希（用于增量编译判断），无行映射表。

后果：`source_map` 模块无法立即提供精确行级断点映射。本次设计采用：
- `SourceMapper` trait 抽象（契约层，立即可用）
- 文件级配对映射（`.rml` ↔ `.rml.rs` 文件配对，无行号翻译）作为 MVP 实现
- 精确行映射留作未来工作（需 engine codegen 增强输出 `.rml.map`，属另一任务）

### 工作区集成点

根 `Cargo.toml`（[Cargo.toml#L2](file:///d:/GitCode/RF/rust-gpui-rml/Cargo.toml#L2)）：
- `members` 需追加 `"crates/dap"`
- `[workspace.dependencies]` 需追加 `rust-rml-dap = { path = "crates/dap" }`

命名遵循项目铁律：`rust-rml-*` 前缀（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-d-GitCode-RF-rust-gpui-rml/project_memory.md)）。

## Proposed Changes

### 1. `crates/dap/Cargo.toml`（新建）

对标 [crates/lsp/Cargo.toml](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/Cargo.toml)：

- `name = "rust-rml-dap"`
- `[lib] name = "rml_dap"`（lib name 去 `rust_` 前缀，与 `rml_lsp` 对齐）
- `description = "RML 调试适配器（DAP）——lldb-dap 引擎集成 + 会话管理 + RML 源映射"`
- 基础依赖：`serde`/`serde_json`（DAP 消息序列化）、`log`、`anyhow`、`lsp-types`（复用
  `Url`/`Position`/`Range` 类型，与 lsp crate 一致）
- `lldb-backend` feature 门：控制 lldb 适配器相关依赖（本期仅 `tokio` 用于子进程 stdio，
  后续可扩展）。feature 关闭时使用 `NoopEngine` 降级

### 2. `crates/dap/README.md`（新建）

文档化以下内容：
- **包职责**：调试引擎集成（lldb-dap）、DAP 协议客户端、调试会话生命周期、RML 源映射
- **非职责**：调试 UI 组件（断点栏/变量树/调用栈面板）属 `rust-rml-ui`；编译器/语法属
  `rust-rml-engine`
- **架构总览**：分层表（对标 lsp crate），中性 trait + feature 门 + 降级
- **集成方式**：spawn `lldb-dap` 子进程，stdio 交流 DAP 协议；`rust-lldb` 作为启动包装
  加载 Rust 符号源路径与 pretty-printer
- **模块结构**：文件树 + 每模块一行职责说明
- **feature 门**：`lldb-backend`（默认关闭，对标 `rust-backend`）
- **RML 源映射**：MVP 文件级配对，精确行映射待 engine 增强
- **使用方式**：demo/app 如何引入并启动调试会话
- **未来工作**：DAP 协议完整实现、源映射精确化、调试 UI 组件

### 3. `crates/dap/src/lib.rs`（新建）

对标 [crates/lsp/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/lib.rs)：
模块声明 + re-export。**仅做模块聚合与 re-export，无业务逻辑**（项目铁律）。

```rust
pub mod engine;
pub mod protocol;
pub mod session;
pub mod source_map;

#[cfg(feature = "lldb-backend")]
pub mod lldb;

pub use engine::{DebugEngine, NoopEngine, ...};
```

### 4. `crates/dap/src/engine.rs`（新建 — 核心契约）

对标 [crates/lsp/src/rust/query.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/query.rs)。
定义 `DebugEngine` trait + 中性调试类型，**零 lldb 依赖**：

- 中性类型：`Breakpoint`/`BreakpointId`/`StackFrame`/`Thread`/`Variable`/`Scope`/
  `StoppedEvent`/`TerminatedEvent`/`DebugState`
- `DebugEngine` trait：`launch`/`attach`/`disconnect`/`set_breakpoints`/
  `continue_`/`step_over`/`step_in`/`step_out`/`pause`/`stack_trace`/`scopes`/
  `variables`/`evaluate`/`is_started`
- `NoopEngine`（非 feature 门，始终可用）：所有方法返回空/未启动，对标
  [rust/mod.rs NoopQuery](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/mod.rs)

### 5. `crates/dap/src/lldb/`（新建 — 引擎适配，feature 门后）

- `mod.rs`：re-export + `#[cfg(feature="lldb-backend")]` 门
- `host.rs`：`LldbHost` 管理 `lldb-dap` 子进程生命周期（spawn/kill/stdio 句柄），
  对标 [rust/host.rs RaHost](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/host.rs)
- `adapter.rs`：`LldbAdapter` 实现 `DebugEngine`，通过 DAP 协议与 `lldb-dap` 通信，
  对标 [rust/adapter.rs RaAdapter](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/adapter.rs)

本期为 **骨架**：类型定义 + trait 实现 stub（`todo!()` 或最小逻辑），可编译通过。
完整 DAP 交互实现留待后续。

### 6. `crates/dap/src/protocol/`（新建 — DAP 协议层）

- `mod.rs`：re-export
- `types.rs`：DAP 协议核心类型（`Request`/`Response`/`Event`/`Message` + 序列号管理 +
  常用命令/事件体），serde 序列化。本期定义协议骨架类型，不依赖外部 dap crate
- `codec.rs`：DAP 消息帧编解码（`Content-Length: N\r\n\r\n<json>` 格式），
  stdio 读写适配

### 7. `crates/dap/src/session/`（新建 — 会话管理）

- `mod.rs`：re-export
- `manager.rs`：`DebugSession` 编排引擎 + 断点 + 状态机（未启动→运行→暂停→终止）
- `breakpoints.rs`：`BreakpointManager` 管理源文件 → 断点列表（增删改查 + 持久化接口）
- `callstack.rs`：调用栈状态（线程/帧缓存，引擎上报后更新）
- `variables.rs`：变量求值与作用域（变量引用树，按 DAP variablesReference 组织）

### 8. `crates/dap/src/source_map/`（新建 — RML 源映射）

- `mod.rs`：re-export
- `mapper.rs`：`SourceMapper` trait + `FilePairMapper`（MVP 文件级配对实现）
  - `rml_to_rust_breakpoint(rml_uri, line) -> Option<(rust_uri, line)>`：
    本期仅返回配对的 `.rml.rs` 文件 + 原始行号（无行号翻译）
  - `rust_to_rml_location(rust_uri, line) -> Option<(rml_uri, line)>`：反向映射
  - 精确行映射标注为 TODO，待 engine codegen 输出 source map 后实现

### 9. 根 `Cargo.toml`（修改）

- `members` 追加 `"crates/dap"`
- `[workspace.dependencies]` 追加 `rust-rml-dap = { path = "crates/dap" }`

## Assumptions & Decisions

1. **脚手架定位**：本次交付 crate 骨架 + 契约 + README，完整 DAP 通信实现留后续。
   trait/类型定义真实可用，适配器实现为 stub。
2. **lldb-dap 子进程方案**（用户确认）：spawn `lldb-dap`（LLVM 自带），stdio 交流 DAP；
   `rust-lldb` 作为启动包装加载 Rust 符号路径。
3. **lib name = `rml_dap`**：去 `rust_` 前缀，与 `rml_lsp` 一致（见 lsp Cargo.toml `[lib] name`）。
4. **复用 `lsp-types` 的 `Url`/`Position`/`Range`**：与 lsp crate 类型一致，避免重复定义
   位置类型，降低跨 crate 转换成本。
5. **`lldb-backend` feature 默认关闭**：对标 `rust-backend`，关闭时 `NoopEngine` 降级，
   保证 crate 无 lldb 依赖也能编译。
6. **源映射 MVP = 文件级配对**：engine codegen 暂无行映射，精确行级断点映射需 engine
   增强（另立任务），本 crate 用 trait 抽象占位。
7. **不引入外部 `dap` crate**：DAP 协议类型手写最小子集（serde），避免引入不成熟依赖
   与版本锁定。后续如需可评估 `debugserver-types` 等。
8. **遵循一文件一职责铁律**：每个 `pub struct`/`pub trait` 独占文件，`mod.rs` 仅 re-export。
9. **调试 UI 组件不在此 crate**：归 `rust-rml-ui`，本 crate 只提供可被 UI 消费的数据模型
   与会话 API。

## Verification Steps

1. **编译验证**：`cargo check -p rust-rml-dap`（无 lldb-backend feature）通过，
   `cargo check -p rust-rml-dap --features lldb-backend` 通过
2. **工作区集成**：`cargo check`（全工作区）通过，新成员被识别
3. **降级验证**：`NoopEngine` 可实例化，所有方法返回安全空值，`is_started()` 返回 false
4. **类型契约**：`DebugEngine` trait 方法签名仅用中性类型（无 lldb/DAP 协议细节泄露）
5. **README 完整性**：职责/非职责/架构/模块/feature/使用方式均覆盖
6. **铁律符合**：`mod.rs` 仅 re-export；每个 `pub struct`/`pub trait` 独占文件

## 不在本次范围

- DAP 协议完整请求/响应实现（`lldb/adapter.rs` 内通信逻辑）
- 调试 UI 组件（断点栏、变量树、调用栈面板）→ 归 `rust-rml-ui`
- engine codegen 输出 source map → 归 `rust-rml-engine` 另一任务
- demo 集成调试会话演示 → 待核心实现完成后再做

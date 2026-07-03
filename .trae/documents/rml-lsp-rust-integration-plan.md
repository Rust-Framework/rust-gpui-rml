# RML LSP 跨语言语法服务架构方案

> 基于 Roslyn 架构原理，引入 rust-analyzer 库依赖，实现 .rml + .rml.rs 的完整跨语言语法服务。

## Context

RML 是类似 WPF 的完整 Rust 项目框架（RML = WPF，Rust = .NET）。当前 LSP 服务器仅支持 .rml 文件的补全/诊断/悬停，.rml.rs 仅作为 StructMetadata 来源被间接消费，无法提供 Rust 语法服务，也无法实现跨语言语义联动（如 .rml 绑定路径 → .rml.rs 字段定义跳转）。

本方案引入 rust-analyzer 的 `ra_ap_*` 库 crate 作为 Rust 语义后端，通过隔离抽象层封装 RA API，确保 RA 变更不污染 LSP 功能代码，同时提供完整的跨语言语义能力。

---

## 1. Roslyn 架构映射

| Roslyn 概念 | RML LSP 实现 |
|-------------|-------------|
| **Workspace** | `workspace::Workspace` — 统一管理 .rml + .rml.rs 文档 + Cargo 项目 |
| **SyntaxTree** | .rml: engine parser 的 `Arc<SyntaxTree>`；.rml.rs: RA 的 rowan 红绿树（封装在 adapter 内） |
| **SemanticModel** | .rml: `semantics::SemanticModel`（binder）；.rml.rs: RA 的 HIR（通过 `RustSemanticQuery` trait 暴露中性接口） |
| **Compilation** | `crosslang::Coordinator` — 聚合 .rml AST + .rml.rs 符号表，统一语义查询 |
| **Symbol** | 中性类型 `SymbolInfo { name, kind, type_str, location }` — 不泄露 RA 类型 |
| **Binding** | `crosslang::resolver` — .rml 绑定路径 → .rml.rs 符号绑定 |
| **Diagnostic** | 三层合并：RML 语法/语义 + RA Rust 诊断 + 跨语言一致性诊断 |

---

## 2. 模块结构

保持现有 `syntax/`/`semantics/`/`features/`/`handlers/`/`server/`/`workspace/` 不动（外科手术式变更），新增 `rust/` 和 `crosslang/` 两个顶层模块：

```
crates/lsp/src/
├── lib.rs                     # 模块声明（新增 rust, crosslang）
├── main.rs                    # 入口（保留）
├── server/                    # LSP 协议层（改造）
│   ├── connection.rs          # ServerState 扩展 rust_query 字段
│   ├── dispatch.rs            # 按文件类型路由（.rml → 本地, .rml.rs → RA）
│   └── conv.rs                # 偏移换算（保留）
├── handlers/                  # LSP 方法处理（扩展）
│   ├── mod.rs                 # 新增 FileType 枚举 + file_type_of(uri)
│   ├── completion.rs          # 按 FileType 分流
│   ├── hover.rs               # 按 FileType 分流
│   ├── diagnostics.rs         # 合并三层诊断
│   └── sync.rs                # .rml.rs 变更 → RA apply_change + 配对 .rml 重诊断
├── features/                  # RML 功能提供器（保留）
├── workspace/                 # 统一 Workspace（扩展 .rml.rs 文档管理 + 自动配对）
├── syntax/                    # RML 语法树（保留）
├── semantics/                 # RML 语义模型（保留）
├── rust/                      # 【新增】RA 集成层（隔离核心）
│   ├── mod.rs
│   ├── query.rs               # RustSemanticQuery trait + 中性类型（隔离层）
│   ├── adapter.rs             # RaAdapter：trait 实现，唯一接触 ra_ap_* 的文件
│   └── host.rs                # AnalysisHost 生命周期 + Cargo workspace 加载
└── crosslang/                 # 【新增】跨语言语义协调层
    ├── mod.rs
    ├── coordinator.rs         # 聚合 RML AST + Rust HIR，统一语义模型
    └── resolver.rs            # .rml 绑定路径 → .rml.rs 符号解析
```

---

## 3. 隔离层设计（核心）

文件：`crates/lsp/src/rust/query.rs`

**设计原则**：所有 `ra_ap_*` 类型（`FileId`/`FilePosition`/`NavigationTarget`/`Analysis` 等）绝不出现在 trait 接口中。中性类型用 `lsp_types` 标准类型（`Url`/`Range`/`Position`），feature 层和 coordinator 只依赖 trait。

```rust
// ── 中性类型（无 RA 依赖）──
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,           // Field / Method / Struct / Enum / Trait
    pub type_str: Option<String>,   // "i32", "String", "Vec<TabItem>"
    pub doc: Option<String>,
    pub location: Option<SymbolLocation>,
}
pub struct SymbolLocation { pub uri: Url, pub range: lsp_types::Range }
pub struct HoverInfo { pub content: String, pub range: Option<lsp_types::Range> }
pub struct CompletionEntry { pub label: String, pub kind: CompletionItemKind, pub detail: Option<String>, pub insert_text: Option<String> }
pub struct RustDiagnostic { pub range: lsp_types::Range, pub severity: DiagnosticSeverity, pub message: String, pub code: Option<String> }

// ── 隔离 trait ── RA API 变更只影响 adapter.rs ──
pub trait RustSemanticQuery: Send + Sync {
    // 文档同步
    fn open_document(&mut self, uri: &Url, text: &str);
    fn apply_change(&mut self, uri: &Url, text: &str);
    fn close_document(&mut self, uri: &Url);

    // .rml.rs 原生 LSP 查询
    fn goto_definition(&self, uri: &Url, pos: Position) -> Vec<SymbolLocation>;
    fn hover(&self, uri: &Url, pos: Position) -> Option<HoverInfo>;
    fn completion(&self, uri: &Url, pos: Position) -> Vec<CompletionEntry>;
    fn diagnostics(&self, uri: &Url) -> Vec<RustDiagnostic>;

    // 跨语言查询（供 coordinator 调用）
    fn resolve_member(&self, rml_rs_uri: &Url, struct_name: &str, member: &str) -> Option<SymbolInfo>;
    fn find_struct(&self, struct_name: &str) -> Option<SymbolLocation>;
    fn struct_slots(&self, rml_rs_uri: &Url, struct_name: &str) -> Vec<String>;
    fn command_signature(&self, rml_rs_uri: &Url, struct_name: &str, method: &str) -> Option<SymbolInfo>;
}
```

**隔离效果**：RA 升级时只需修改 `adapter.rs` 一个文件中的类型转换函数（如 `nav_to_location`），LSP 功能代码零改动。

---

## 4. RA 适配器

文件：`crates/lsp/src/rust/host.rs` + `adapter.rs`

### host.rs — Cargo workspace 加载

`initialize` 握手后用 `root_uri` 调 `ra_ap_load_cargo::load_cargo` 构建 `AnalysisHost` + `HashMap<Url, FileId>` 映射。`LoadCargoConfig` 启用 proc-macro server（`#[window]`/`#[component]` 属性宏可被 RA 识别）。

首次加载耗时较长（30s+），在 `initialized` 通知后异步加载，加载完成前 `.rml.rs` 查询降级返回空结果。

### adapter.rs — trait 实现

- `apply_change`：`host.analysis.apply_change(Change::set_text(file_id, text))`
- `resolve_member`：用 `Semantics` 定位 struct → 遍历 fields/methods 匹配名称 → 提取 `hir::Field`/`hir::Function` 的类型字符串
- `find_struct`：全 workspace 搜索 struct def → `NavigationTarget` 转 `SymbolLocation`
- `goto_definition`/`hover`/`completion`：委托 `Analysis::goto_definition`/`hover`/`completions`，结果转中性类型
- `command_signature`：查找 `#[command]` 标注方法 → 提取参数类型

所有 RA 类型转换函数集中在 adapter.rs 内部，不外泄。

---

## 5. 跨语言协调器

文件：`crates/lsp/src/crosslang/coordinator.rs` + `resolver.rs`

无状态函数模块，每个函数接收 `&Workspace` + `&dyn RustSemanticQuery`。

| 函数 | 职责 | 数据流 |
|------|------|--------|
| `resolve_binding` | 绑定路径类型解析 | .rml `{user.name}` → ProjectIndex 取配对 struct → `query.resolve_member()` 获取类型 |
| `validate_component_tag` | 组件标签校验 | .rml `<MyWidget>` → `query.find_struct("MyWidget")` 确认存在 + `#[component]` 属性 |
| `validate_slot` | slot 合法性校验 | .rml `<template slot="x">` → `query.struct_slots()` 与 StructMetadata.slots 交叉验证 |
| `validate_root_tag` | 根标签 ↔ 宏属性一致性 | .rml `<window>` → .rml.rs `#[window]`（当前 scanner 不校验，仅 codegen 报错） |
| `goto_def_from_rml` | 跨文件跳转 | .rml `{field}` 光标 → `query.resolve_member()` 返回 `SymbolLocation` → .rml.rs 字段声明 |
| `command_completion` | 命令补全 | `StructMetadata.commands` + `query.command_signature()` 提供参数签名 |

现有 `semantics/binder.rs` 的 `check_binding_expr` 保留为快速存在性检查；coordinator 在其之上叠加类型推断与跨文件导航。

---

## 6. LSP 分发改造

### 文件类型路由

`handlers/mod.rs` 新增：
```rust
pub enum FileType { Rml, RmlRs, Other }
pub fn file_type_of(uri: &Url) -> FileType { /* 按 .rml / .rml.rs 扩展名判断 */ }
```

`dispatch.rs` 的 `handle_request`/`handle_notification` 内部按 `FileType` 分流到对应 handler。

### ServerState 扩展

```rust
pub struct ServerState {
    pub workspace: Workspace,
    pub rust_query: Box<dyn RustSemanticQuery>,
    pub shutdown_requested: bool,
}
```

### sync.rs 改造

`handle_did_open`/`handle_did_change` 按 FileType：
- `.rml`：现有逻辑 + coordinator 增强诊断
- `.rml.rs`：`rust_query.apply_change()` + `workspace.refresh_codebehind()` + 反向查找配对 .rml 触发重诊断

### ServerCapabilities

开启 `definition_provider`、`references_provider`，`document_sync` 保持 FULL。

---

## 7. Cargo.toml 变更

`crates/lsp/Cargo.toml` 新增（统一锁定到同一 rev 避免版本错配）：

```toml
[dependencies]
ra_ap_ide = { git = "https://github.com/rust-lang/rust-analyzer.git", rev = "<锁定rev>" }
ra_ap_load-cargo = { git = "https://github.com/rust-lang/rust-analyzer.git", rev = "<锁定rev>" }
ra_ap_hir = { git = "https://github.com/rust-lang/rust-analyzer.git", rev = "<锁定rev>" }
ra_ap_vfs = { git = "https://github.com/rust-lang/rust-analyzer.git", rev = "<锁定rev>" }
ra_ap_project-model = { git = "https://github.com/rust-lang/rust-analyzer.git", rev = "<锁定rev>" }

[features]
default = ["rust-backend"]
rust-backend = ["ra_ap_ide", "ra_ap_load-cargo", "ra_ap_hir", "ra_ap_vfs", "ra_ap_project-model"]
```

feature flag 允许降级到纯 syn 扫描模式（无 RA 依赖），便于 CI 快速编译。

---

## 8. 实施阶段

### Phase 1 — 隔离层骨架
- 新建 `rust/query.rs`（trait + 中性类型）、`rust/mod.rs`
- `Cargo.toml` 加 RA git 依赖
- `ServerState` 加 `rust_query` 字段（`NoopQuery` 空实现）
- **验证**：`cargo build -p rust-rml-lsp` 通过，trait + 中性类型编译无 RA 类型泄露

### Phase 2 — RA 适配器
- `rust/host.rs`：`load_cargo` 集成 + AnalysisHost 生命周期
- `rust/adapter.rs`：实现 `open_document`/`apply_change`/`diagnostics`/`hover`/`goto_definition`/`completion`
- `handlers/sync.rs`：`.rml.rs` 变更路由到 adapter
- `dispatch.rs`：按文件类型分流
- **验证**：打开 demo 的 `button_case.rml.rs`，RA 诊断 + 补全 + 悬停正常返回

### Phase 3 — 跨语言协调器
- `crosslang/resolver.rs`：绑定路径 → 符号解析
- `crosslang/coordinator.rs`：`resolve_binding` + `validate_component_tag` + `validate_root_tag`
- `handlers/diagnostics.rs`：合并跨语言诊断
- `handlers/sync.rs`：`.rml.rs` 变更触发配对 `.rml` 重诊断
- `workspace/project_index.rs`：initialize 时自动扫描配对
- **验证**：`button_case.rml` 中 `{count}` 可跳转到 `button_case.rml.rs` 字段声明

### Phase 4 — 深度语义
- `resolve_member` 返回类型信息 → binder 类型检查
- `struct_slots` → slot 校验
- `command_signature` → 命令方法签名 hover + 补全
- 组件标签补全从 RA 动态获取 `#[component]` struct 列表
- **验证**：`onclick={on_click}` 悬停显示方法签名；`<MyWidget>` 补全动态获取用户组件

---

## 9. 关键风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| RA API 不稳定（0.0.x） | adapter 编译失败 | 隔离 trait 使影响范围限定在 `adapter.rs` 单文件；锁定 rev |
| 编译时间剧增（8-15min） | 开发体验 | feature flag `rust-backend` 可降级为纯 syn 模式；`[profile.dev]` 优化 RA 包 |
| proc-macro server 依赖 | `#[window]` 展开失败 | 字段/方法签名在展开前即可见，基础查询不依赖展开；降级过滤 RA 的 "missing impl" 噪声 |
| 内存占用（1-2GB） | 大型 workspace | `load_cargo` 配置禁用不必要的特性 |
| 首次加载阻塞（30s+） | initialize 卡顿 | `initialized` 后异步加载，加载完成前 .rml.rs 查询返回空 |
| .rml.rs 扩展名识别 | RA 默认只识别 .rs | .rml.rs 本身是 .rs 子串，RA 原生支持；LSP 层按 `ends_with(".rml.rs")` 路由 |

---

## 10. 验证步骤

1. `cargo build -p rust-rml-lsp` — 全特性编译通过
2. `cargo build -p rust-rml-lsp --no-default-features` — 降级模式（无 RA）编译通过
3. `cargo test -p rust-rml-lsp` — 隔离层中性类型 + conv 换算测试
4. `cargo build -p rust-rml-demo` — workspace 不破坏
5. `cargo clippy -p rust-rml-lsp` — 零警告
6. **端到端**：LSP 客户端打开 `demo/src/cases/button_case.rml.rs`，验证 RA 补全/诊断/悬停
7. **跨语言**：在 `button_case.rml` 的 `{count}` 上触发 goto definition，跳转到 `.rml.rs` 字段
8. **联动刷新**：修改 `.rml.rs` 的 `#[command]` 方法名，`.rml` 的 `onclick={old_name}` 立即报诊断

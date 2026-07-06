# LanguageClient 封装设计计划

## 摘要

在 `rust-rml-client` crate 中封装 `LanguageClient` 高级结构体，内部统一管理：
1. **LSP 服务端进程生命周期**（spawn / initialize / shutdown）
2. **LSP 协议通信**（JSON-RPC over stdio，经内部 `LspClient`）
3. **CodeEditor provider 工厂**（completion / hover / definition / semantic_tokens）
4. **Tree-sitter 静态语法注册**（可选，语言自定义 grammar 时）

同一设计模式预留 `DebugClient` 骨架（DAP 协议，后续实现）。

**核心目标**：高内聚低耦合 —— demo 侧只需 `LanguageClient::rml(workspace_root)?` 一行启动，`client.install_providers(&mut state, uri)` 一行集成所有 providers。支持任意语言（RML / Rust / …）。

---

## 当前状态分析

### rust-rml-client crate 现状（Phase B 部分完成）

`crates/rml/src/` 目录结构：

| 文件 | 职责 | 问题 |
|------|------|------|
| `lib.rs` | 模块声明 + 顶层 re-exports | 导出散落的 free functions |
| `grammar.rs` | RML tree-sitter grammar（C ABI + 查询字符串） | RML 专属，正常 |
| `lsp_client.rs` | `LspClient` struct（IPC + 子进程） | `resolve_binary()` 硬编码 `"rml-lsp"` |
| `registry.rs` | `register_rml_language()` free function | RML 专属，应并入 `LanguageClient` |
| `editor.rs` | `install_lsp_providers()` free function | 应并入 `LanguageClient` |
| `prelude.rs` | 便捷 re-exports | 导出旧 API |
| `providers/completion.rs` | `RmlCompletionProvider` | 命名含 `Rml`，但实现语言无关 |
| `providers/hover.rs` | `RmlHoverProvider` | 同上 |
| `providers/definition.rs` | `RmlDefinitionProvider` | 同上 |
| `providers/semantic_tokens.rs` | `RmlSemanticTokensProvider` | 同上 |

### demo 现状（BROKEN — 编译失败）

5 个文件已从 `demo/src/lsp/` 移出至 `crates/rml/src/`，但 demo 仍引用旧路径：

| 文件 | 破损点 |
|------|--------|
| `demo/Cargo.toml:15` | `tree-sitter-rml = { workspace = true }`（包已更名） |
| `demo/src/app.rs:8` | `use tree_sitter_rml::{language, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};` |
| `demo/src/lsp/mod.rs:5-13` | `pub mod lsp_client; pub mod completion_provider; …`（文件已移走） |
| `demo/src/lsp/code_editor_tab.rml.rs:18-21` | `use crate::lsp::{RmlCompletionProvider, …};`（已移至 `rml::providers`） |
| `demo/src/shell/main_window.rml.rs:16,140` | `use crate::lsp::LspClient;` + `LspClient::spawn(…)` |
| `demo/src/shell/workbench.rs:21` | `use crate::lsp::{CodeEditorTab, LspClient};` |

### gpui-component API 关键点

- `LanguageRegistry::singleton()` 在 `LazyLock::new` 时已注入 `Language::all()` —— 启用 `tree-sitter-languages` feature 后，**Rust 语法已自动注册**（`tree_sitter_rust::LANGUAGE`）
- `LanguageConfig::new(name, language, injection_languages, highlights, injections, locals)`
- `Lsp` struct 字段：`completion_provider / hover_provider / definition_provider / semantic_tokens_provider` 等，类型为 `Option<Rc<dyn Trait>>`
- workspace `Cargo.toml` 已启用 `features = ["tree-sitter-languages"]`

### 当前 demo LSP 集成链路

```
MainWindow::init_lsp() 
  → LspClient::spawn(workspace_root)       [硬编码 rml-lsp]
  → self.lsp_client: Option<Arc<LspClient>>
  → LspWorkbenchProvider::new(lsp_client)
  → LspWorkbench::new(uri, title, lsp_client)
  → CodeEditorTab::new(file_path, full_path, lsp_client, window, cx)
    → InputState::new().code_editor(language)
    → 手动安装 4 个 Rml*Provider
```

**缺陷**：单一 `LspClient` 服务所有文件，但 `CodeEditorTab` 检测 `.rs` 文件时传 `language="rust"` 给 rml-lsp —— 实际 rml-lsp 不懂 Rust。需按语言分离 client。

---

## 设计方案

### 1. `LanguageProfile` —— 语言配置预设

```rust
/// Tree-sitter 语法包（可选 —— 内置语言无需提供）
pub struct TreeSitterGrammar {
    pub language: tree_sitter::Language,
    pub highlights: &'static str,
    pub injections: &'static str,
    pub locals: &'static str,
    pub injection_languages: Vec<SharedString>,
}

/// 语言服务配置 —— 描述如何启动并与某语言的 LSP server 交互
#[derive(Clone)]
pub struct LanguageProfile {
    pub language_id: SharedString,          // "rml" / "rust"
    pub file_extensions: Vec<SharedString>, // ["rml"] / ["rs"]
    pub grammar: Option<TreeSitterGrammar>, // None = 依赖 gpui-component 内置
    pub server_binary: String,              // "rml-lsp" / "rust-analyzer"
    pub server_args: Vec<String>,           // ["--stdio"]
    pub server_path_env: Option<&'static str>, // "RML_LSP_PATH" / None
    pub server_search_paths: Vec<&'static str>, // 相对 workspace_root
}

impl LanguageProfile {
    pub fn rml() -> Self { /* grammar=Some, binary="rml-lsp", args=["--stdio"] */ }
    pub fn rust() -> Self { /* grammar=None, binary="rust-analyzer", args=["--stdio"] */ }
}
```

**决策**：
- `grammar: Option` —— RML 自带 grammar；Rust 依赖 gpui-component 内置（`tree-sitter-languages` feature 已启用）
- `server_path_env` —— 允许环境变量覆盖二进制路径（开发期灵活）
- `LanguageProfile::rml()` 引用 `crate::grammar::language() / HIGHLIGHTS_QUERY / INJECTIONS_QUERY`

### 2. `LanguageClient` —— 高级语言服务客户端

```rust
/// LanguageClient —— 高内聚语言服务客户端
///
/// 封装 LSP server 进程 + IPC + provider 工厂 + grammar 注册。
/// 一个实例服务一种语言；多语言场景创建多个实例。
pub struct LanguageClient {
    profile: LanguageProfile,
    lsp: Arc<LspClient>,
}

impl LanguageClient {
    /// 通用构造：按 profile 启动 LSP server 并完成 initialize 握手
    pub fn new(profile: LanguageProfile, workspace_root: &Path) -> Result<Self> {
        // 1. 注册 tree-sitter grammar（若 profile.grammar == Some）
        // 2. spawn LSP server（profile 驱动二进制解析）
        // 3. LSP initialize 握手
    }

    /// RML 便捷构造
    pub fn rml(workspace_root: &Path) -> Result<Self> {
        Self::new(LanguageProfile::rml(), workspace_root)
    }

    /// Rust 便捷构造（rust-analyzer）
    pub fn rust(workspace_root: &Path) -> Result<Self> {
        Self::new(LanguageProfile::rust(), workspace_root)
    }

    /// 打开文档（自动用 profile.language_id）
    pub fn open_document(&self, uri: &Uri, text: &str) {
        self.lsp.open_document(uri, text, &self.profile.language_id);
    }

    /// 文档变更通知
    pub fn change_document(&self, uri: &Uri, text: &str) {
        self.lsp.change_document(uri, text);
    }

    /// 一行安装所有 LSP providers 到 InputState（绑定到指定 URI）
    pub fn install_providers(&self, state: &mut InputState, uri: Uri) {
        state.lsp.completion_provider = Some(Rc::new(LspCompletionProvider::new(self.lsp.clone(), uri.clone())));
        state.lsp.hover_provider = Some(Rc::new(LspHoverProvider::new(self.lsp.clone(), uri.clone())));
        state.lsp.definition_provider = Some(Rc::new(LspDefinitionProvider::new(self.lsp.clone(), uri.clone())));
        if let Some(legend) = self.lsp.semantic_tokens_legend() {
            state.lsp.semantic_tokens_provider = Some(Rc::new(LspSemanticTokensProvider::new(self.lsp.clone(), uri, legend)));
        }
    }

    /// 直访底层 LspClient（formatting / rename / references / document_symbol 等）
    pub fn lsp(&self) -> &LspClient { &self.lsp }

    /// 语言 profile
    pub fn profile(&self) -> &LanguageProfile { &self.profile }
}
```

### 3. `LspClient` 重构（内部细节）

`LspClient::spawn` 签名变更：

```rust
// 旧：pub fn spawn(workspace_root: &Path) -> Result<Self>
// 新：pub fn spawn(profile: &LanguageProfile, workspace_root: &Path) -> Result<Self>
```

`resolve_binary()` 泛化：

```rust
// 旧：硬编码 "rml-lsp" / "rml-lsp.exe" / target / crates/lsp/target
// 新：fn resolve_binary(profile: &LanguageProfile, workspace_root: &Path) -> Result<PathBuf>
//   - 先查 profile.server_path_env 环境变量
//   - 再查 profile.server_search_paths 下的 profile.server_binary[.exe]
//   - 最后回退 PATH 查找 profile.server_binary
```

### 4. Provider 重命名（语言无关化）

| 旧名 | 新名 |
|------|------|
| `RmlCompletionProvider` | `LspCompletionProvider` |
| `RmlHoverProvider` | `LspHoverProvider` |
| `RmlDefinitionProvider` | `LspDefinitionProvider` |
| `RmlSemanticTokensProvider` | `LspSemanticTokensProvider` |

实现不变，仅改名（providers 本就不含 RML 专属逻辑）。

### 5. `DebugClient` 骨架（同设计模式，后续实现）

```rust
/// DebugClient —— 调试服务客户端（DAP 协议）
///
/// 与 LanguageClient 同构：封装 debug adapter 进程 + DAP IPC + 调试能力工厂。
/// 当前为骨架，后续实现 DAP 协议后补齐方法。
pub struct DebugClient {
    profile: DebugProfile,
    // dap: Arc<DapClient>,  // 后续引入
}

#[derive(Clone)]
pub struct DebugProfile {
    pub language_id: SharedString,      // "rust"
    pub adapter_binary: String,         // "codelldb" / "lldb-vscode"
    pub adapter_args: Vec<String>,
    pub adapter_path_env: Option<&'static str>,
}

impl DebugProfile {
    pub fn rust() -> Self { /* codelldb 或 lldb-vscode */ }
}

impl DebugClient {
    pub fn new(profile: DebugProfile, workspace_root: &Path) -> Result<Self> {
        // 后续：spawn DAP adapter + attach
        todo!("DAP implementation in future phase")
    }
    pub fn rust(workspace_root: &Path) -> Result<Self> {
        Self::new(DebugProfile::rust(), workspace_root)
    }
}
```

仅创建 `debug_client.rs` 文件含 struct + 方法签名（`todo!`），不引入 DAP 依赖。

### 6. 模块布局（目标）

```
crates/rml/src/
├── lib.rs                  # 公共 API: LanguageClient, LanguageProfile, DebugClient, DebugProfile
├── prelude.rs              # 便捷 re-exports
├── grammar.rs              # RML tree-sitter grammar（RML 专属，不变）
├── language_client.rs      # LanguageClient + LanguageProfile + TreeSitterGrammar (NEW)
├── debug_client.rs         # DebugClient + DebugProfile 骨架 (NEW)
├── lsp_client.rs           # LspClient (重构: spawn 取 LanguageProfile)
├── language_profile.rs     # LanguageProfile / TreeSitterGrammar / DebugProfile 定义 (NEW)
└── providers/              # Lsp*Provider（重命名自 Rml*Provider）
    ├── mod.rs
    ├── completion.rs       # LspCompletionProvider
    ├── hover.rs            # LspHoverProvider
    ├── definition.rs       # LspDefinitionProvider
    └── semantic_tokens.rs  # LspSemanticTokensProvider
```

**删除**：`registry.rs`（并入 `LanguageClient::new`）、`editor.rs`（并入 `LanguageClient::install_providers`）

### 7. demo 集成 API（修复破损状态）

#### `demo/Cargo.toml`
```toml
# 旧：tree-sitter-rml = { workspace = true }
# 新：
rust-rml-client = { workspace = true }
# 移除冗余：lsp-server, lsp-types, url, ropey, crossbeam-channel（由 rust-rml-client 传递）
```

#### `demo/src/app.rs`
```rust
// 旧：use tree_sitter_rml::{language, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};
//         LanguageRegistry::singleton().register(...)
// 新：完全移除 —— grammar 注册由 LanguageClient::rml() 在 init_lsp 时完成
```

#### `demo/src/shell/main_window.rml.rs`
```rust
// 旧：use crate::lsp::LspClient;
//         LspClient::spawn(&workspace_root)
// 新：use rust_rml_client::LanguageClient;
//         LanguageClient::rml(&workspace_root)
```

`MainWindow` 字段：
```rust
// 旧：lsp_client: Option<Arc<LspClient>>,
// 新：language_client: Option<Arc<LanguageClient>>,
```

#### `demo/src/lsp/mod.rs`
```rust
// 移除已移走模块声明，仅保留 demo 专属：
pub mod code_editor_tab;
pub mod file_tree;
pub mod lsp_status;
#[path = "lsp_explorer_panel.rml.rs"]
pub mod lsp_explorer_panel;

pub use code_editor_tab::CodeEditorTab;
pub use lsp_status::{ensure_lsp_status_item_registered, LspStatusState, LspStatusStateRef};
```

#### `demo/src/lsp/code_editor_tab.rml.rs`
```rust
// 旧：use crate::lsp::{RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider, RmlSemanticTokensProvider};
//         手动安装 4 个 provider
// 新：use rust_rml_client::LanguageClient;
//         language_client.install_providers(&mut state, uri);
```

字段 `lsp_client: Option<Arc<LspClient>>` → `language_client: Option<Arc<LanguageClient>>`，方法调用 `client.open_document()` / `client.change_document()` / `client.lsp().formatting()` 等保持兼容。

---

## 实施步骤

### Step 1: 创建 `language_profile.rs`（新文件）

定义 `LanguageProfile`、`TreeSitterGrammar`、`DebugProfile`。
`LanguageProfile::rml()` 引用 `crate::grammar::{language, HIGHLIGHTS_QUERY, INJECTIONS_QUERY}`。
`LanguageProfile::rust()` 设 `grammar: None`（依赖 gpui-component 内置）。

### Step 2: 重构 `lsp_client.rs`

- `LspClient::spawn(profile: &LanguageProfile, workspace_root: &Path)` —— 用 `profile.server_binary / server_args / server_path_env / server_search_paths`
- `resolve_binary(profile, workspace_root)` —— 泛化版本
- `open_document` 保持原签名（`language_id: &str` 由 `LanguageClient` 传入）

### Step 3: 重命名 providers（`Rml*` → `Lsp*`）

4 个文件：
- `providers/completion.rs`: `RmlCompletionProvider` → `LspCompletionProvider`
- `providers/hover.rs`: `RmlHoverProvider` → `LspHoverProvider`
- `providers/definition.rs`: `RmlDefinitionProvider` → `LspDefinitionProvider`
- `providers/semantic_tokens.rs`: `RmlSemanticTokensProvider` → `LspSemanticTokensProvider`
- `providers/mod.rs`: 更新 re-exports

### Step 4: 创建 `language_client.rs`（新文件）

实现 `LanguageClient` struct + `new()` / `rml()` / `rust()` / `open_document()` / `change_document()` / `install_providers()` / `lsp()` / `profile()`。

### Step 5: 创建 `debug_client.rs`（骨架）

`DebugClient` + `DebugProfile`，方法体 `todo!()`。

### Step 6: 删除 `registry.rs` + `editor.rs`

功能已并入 `LanguageClient`。

### Step 7: 更新 `lib.rs` + `prelude.rs`

```rust
// lib.rs
pub mod debug_client;
pub mod grammar;
pub mod language_client;
pub mod language_profile;
pub mod lsp_client;
pub mod prelude;
pub mod providers;

pub use language_client::LanguageClient;
pub use language_profile::{DebugProfile, LanguageProfile, TreeSitterGrammar};
pub use debug_client::DebugClient;
pub use lsp_client::{file_path_to_uri, LspClient};
pub use providers::{
    LspCompletionProvider, LspDefinitionProvider, LspHoverProvider, LspSemanticTokensProvider,
};

// prelude.rs
pub use crate::language_client::LanguageClient;
pub use crate::language_profile::{DebugProfile, LanguageProfile};
pub use crate::lsp_client::file_path_to_uri;
pub use crate::providers::*;
```

移除 `register_rml_language` / `install_lsp_providers` 导出（已内化）。

### Step 8: 编译 `rust-rml-client` 独立验证

`cargo check -p rust-rml-client` 通过。

### Step 9: 修复 `demo/Cargo.toml`

替换 `tree-sitter-rml` → `rust-rml-client`，移除冗余 LSP 依赖。

### Step 10: 修复 `demo/src/app.rs`

移除 `use tree_sitter_rml::{...}` 和手动 `LanguageRegistry::register(...)`。

### Step 11: 修复 `demo/src/lsp/mod.rs`

移除已移走模块声明，保留 demo 专属模块。

### Step 12: 修复 `demo/src/lsp/code_editor_tab.rml.rs`

- 导入改为 `use rust_rml_client::LanguageClient;`
- 字段 `lsp_client` → `language_client`
- `LspClient` → `LanguageClient`
- 手动安装 4 provider → `language_client.install_providers(&mut state, uri)`
- `lsp_client.open_document(&uri, &text, language)` → `language_client.open_document(&uri, &text)`
- `client.formatting(&uri)` → `client.lsp().formatting(&uri)`（其他直访方法同理）

### Step 13: 修复 `demo/src/shell/main_window.rml.rs`

- 导入 `use rust_rml_client::LanguageClient;`
- 字段 `lsp_client` → `language_client: Option<Arc<LanguageClient>>`
- `init_lsp()`: `LanguageClient::rml(&workspace_root)`
- 传递 `language_client` 给 `LspWorkbenchProvider` / `LspWorkbench`

### Step 14: 修复 `demo/src/shell/workbench.rs`

- `use crate::lsp::{CodeEditorTab, LspClient}` → `use crate::lsp::CodeEditorTab; use rust_rml_client::LanguageClient;`
- `LspWorkbench.lsp_client: Option<Arc<LspClient>>` → `Option<Arc<LanguageClient>>`
- `LspWorkbenchProvider` 同步改字段类型

### Step 15: 编译 + 测试验证

- `cargo check -p rust-rml-client` —— crate 独立编译通过
- `cargo check -p rust-rml-demo` —— demo 编译通过
- `cargo test -p rust-rml-client` —— grammar 单元测试通过
- 手动运行 demo，确认 LSP 案例（CodeEditorTab）静态着色 + 动态语义 token 正常

---

## 假设与决策

### 假设

1. **Rust tree-sitter grammar 已由 gpui-component 内置**：workspace `Cargo.toml` 已启用 `features = ["tree-sitter-languages"]`，`LanguageRegistry::singleton()` 初始化时调用 `Language::all()` 自动注册 Rust。`LanguageProfile::rust()` 设 `grammar: None` 即可。

2. **rust-analyzer 二进制可能不在 PATH**：`LanguageProfile::rust()` 的 `resolve_binary()` 先查环境变量 `RA_PATH`（若有），再查 PATH，找不到时 `LanguageClient::rust()` 返回 `Err`，demo 侧优雅降级（`init_lsp` 已有 `log::warn` 兜底）。

3. **`DebugClient` 仅骨架**：用户明确说"后续DebugClient也一样的设计"，本计划只创建 struct + 方法签名（`todo!`），不引入 DAP 依赖，不实现协议。

4. **demo 仅启用 RML client**：当前 demo 仅编辑 `.rml` 文件实际走 LSP；`.rs` 文件虽检测 `language="rust"` 但 rml-lsp 不懂 Rust。本计划保持 demo 只 spawn `LanguageClient::rml()`，`.rs` 文件的 Rust LSP 集成留待后续（需 rust-analyzer 可用）。`CodeEditorTab` 的 language 检测逻辑保留，rml-lsp 会按 language_id 路由（若 rml-lsp 不支持 rust 文档则 LSP 功能静默失效，不影响静态着色）。

### 决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| `LanguageClient` 一实例一语言 | ✓ | 高内聚低耦合；多语言多实例 |
| Provider 重命名 `Rml*` → `Lsp*` | ✓ | 实现本就语言无关 |
| `grammar: Option<TreeSitterGrammar>` | ✓ | 内置语言无需重复注册 |
| `DebugClient` 骨架而非完整实现 | ✓ | 用户明确"后续" |
| 删除 `registry.rs` + `editor.rs` | ✓ | 功能内化到 `LanguageClient`，避免散落 free function |
| `LspClient` 保留 `pub` | ✓ | `LanguageClient::lsp()` 直访底层 IPC（formatting/rename/references 等不常用方法不重复封装） |
| demo 不引入 `LanguageClient::rust()` | ✓ | rust-analyzer 可用性未确认；保持现状最小修复 |

---

## 验证步骤

1. **crate 独立编译**：`cargo check -p rust-rml-client` —— 0 error
2. **demo 编译**：`cargo check -p rust-rml-demo` —— 0 error（允许既有 dead_code 警告）
3. **grammar 测试**：`cargo test -p rust-rml-client` —— 5 个 grammar 测试通过
4. **运行时验证**（手动）：
   - 启动 demo，打开 LSP Explorer → 点击 `.rml` 文件
   - CodeEditorTab 显示代码：tree-sitter 静态着色（tag/attribute/keyword/string 等）
   - 100ms 后 LSP 动态语义 token 覆盖（resolved/unresolved field、type、function）
   - 编辑代码 → `change_document` 通知 → semantic token 增量更新
   - 触发 completion / hover / definition 验证 LSP 功能正常
5. **回归**：既有案例页面（accordion_case 等 16 个）不受影响

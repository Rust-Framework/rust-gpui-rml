# rust-rml-client 客户端整合 crate 重构计划

## Context

当前 RML 客户端逻辑分散在两处：
- `crates/tree-sitter-rml`：仅含 tree-sitter 语法 + 静态着色查询
- `demo/src/lsp/`：含 LSP 客户端进程管理、4 个 gpui-component provider 实现、CodeEditorTab UI、状态栏、文件树等

demo 想集成语法服务必须复制 `LspClient` + 4 个 provider 文件并手写 7+ 行安装代码，复用性为零。同时后续 DAP 调试客户端也面临同样问题。

本次重构将所有**可复用的客户端逻辑**（语法、LSP 客户端、providers、editor 集成助手）统一到 `crates/rml`（包名 `rust-rml-client`）。demo 仅保留 UI 相关代码（CodeEditorTab、状态栏、文件树、Explorer），通过一行 `install_lsp_providers(...)` + 一行 `register_rml_language()` 完成集成。DAP 客户端未来也归入此 crate。

## 目标 API（demo 侧集成代码）

`demo/src/app.rs`：
```rust
rust_rml_client::register_rml_language();
```

`demo/src/lsp/code_editor_tab.rml.rs`：
```rust
use rust_rml_client::{file_path_to_uri, install_lsp_providers, LspClient};

let editor_state = cx.new(|cx| {
    let mut state = InputState::new(window, cx)
        .code_editor(language)
        .multi_line(true)
        .tab_size(TabSize { tab_size: 4, ..Default::default() })
        .default_value(&text);
    install_lsp_providers(&mut state, lsp_client.clone(), uri.clone());
    state
});
```

原 7 行 provider 安装代码（4 个 `Some(Rc::new(...))` + 1 个 `if let Some(legend)`）压缩为 1 行。

## 新 crate 目录结构

```
crates/rml/                          (重命名自 crates/tree-sitter-rml/)
├── Cargo.toml                       (package = "rust-rml-client")
├── build.rs                         (不变，编译 src/parser.c)
├── grammar.js                       (不变，tree-sitter CLI 源)
├── package.json                     (不变)
├── queries/
│   ├── highlights.scm               (不变)
│   └── injections.scm               (不变)
└── src/
    ├── lib.rs                       (新 — 模块声明 + re-exports)
    ├── grammar.rs                   (移自原 lib.rs — language() + 3 个 query 常量 + 5 个测试)
    ├── lsp_client.rs                (移自 demo/src/lsp/lsp_client.rs — LspClient + file_path_to_uri)
    ├── registry.rs                  (新 — register_rml_language())
    ├── editor.rs                    (新 — install_lsp_providers())
    ├── prelude.rs                   (新 — 便捷 re-exports)
    ├── providers/
    │   ├── mod.rs                   (新 — re-exports)
    │   ├── completion.rs            (移自 demo — RmlCompletionProvider)
    │   ├── hover.rs                 (移自 demo — RmlHoverProvider)
    │   ├── definition.rs            (移自 demo — RmlDefinitionProvider)
    │   └── semantic_tokens.rs       (移自 demo — RmlSemanticTokensProvider)
    ├── parser.c                     (不变，tree-sitter 生成)
    ├── grammar.json                 (不变)
    ├── node-types.json              (不变)
    └── tree_sitter/                 (不变 — alloc.h / array.h / parser.h)
```

**关键迁移**：`lib.rs` 内容（tree-sitter 入口）拆到 `src/grammar.rs`，原 `include_str!("queries/highlights.scm")` 路径改为 `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/queries/highlights.scm"))`（`include_str!` 相对当前文件，移动后路径需用 `CARGO_MANIFEST_DIR` 锚定到 crate 根）。

`Cargo.toml` 删除 `[lib] path = "lib.rs"`，使用默认 `src/lib.rs`。

## 实施步骤

### Step 1: 重命名 crate 目录

`crates/tree-sitter-rml/` → `crates/rml/`（`git mv` 或文件系统移动整个目录）。

### Step 2: 重构 crate 内文件布局

1. 创建 `src/` 子目录（若已有 `src/parser.c` 等则在原地保留）
2. 创建 `src/lib.rs`：声明 `pub mod grammar; pub mod lsp_client; pub mod providers; pub mod editor; pub mod registry; pub mod prelude;` + 顶层 re-exports
3. 创建 `src/grammar.rs`：移入原 `lib.rs` 内容（`language()`、`HIGHLIGHTS_QUERY`、`INJECTIONS_QUERY`、5 个测试），更新 `include_str!` 路径为 `concat!(env!("CARGO_MANIFEST_DIR"), "/queries/...")`
4. 删除根目录 `lib.rs`

### Step 3: 移入 LSP 客户端与 providers

1. `demo/src/lsp/lsp_client.rs` → `crates/rml/src/lsp_client.rs`
   - 修改内部 import：`use crate::lsp::LspClient;` → 删除（同模块内）
2. `demo/src/lsp/completion_provider.rs` → `crates/rml/src/providers/completion.rs`
   - 修改 import：`use crate::lsp::LspClient;` → `use crate::lsp_client::LspClient;`
3. `demo/src/lsp/hover_provider.rs` → `crates/rml/src/providers/hover.rs`（同上 import 修改）
4. `demo/src/lsp/definition_provider.rs` → `crates/rml/src/providers/definition.rs`（同上）
5. `demo/src/lsp/semantic_tokens_provider.rs` → `crates/rml/src/providers/semantic_tokens.rs`
   - 修改 import：`use crate::lsp::LspClient;` → `use crate::lsp_client::LspClient;`
6. 创建 `crates/rml/src/providers/mod.rs`：re-export 4 个 provider

### Step 4: 创建 registry.rs

```rust
//! 一行注册 RML 语言到 gpui-component LanguageRegistry（静态着色）。

pub fn register_rml_language() {
    use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
    LanguageRegistry::singleton().register(
        "rml",
        &LanguageConfig::new(
            "rml",
            tree_sitter::Language::new(crate::grammar::language()),
            vec!["rust".into()],
            crate::grammar::HIGHLIGHTS_QUERY,
            crate::grammar::INJECTIONS_QUERY,
            "",
        ),
    );
}
```

### Step 5: 创建 editor.rs

```rust
//! 一行集成所有 RML LSP providers 到 InputState。

use std::rc::Rc;
use std::sync::Arc;

use gpui_component::input::InputState;
use lsp_types::Uri;

use crate::lsp_client::LspClient;
use crate::providers::{
    RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider, RmlSemanticTokensProvider,
};

pub fn install_lsp_providers(
    state: &mut InputState,
    client: Arc<LspClient>,
    uri: Uri,
) {
    state.lsp.completion_provider =
        Some(Rc::new(RmlCompletionProvider::new(client.clone(), uri.clone())));
    state.lsp.hover_provider =
        Some(Rc::new(RmlHoverProvider::new(client.clone(), uri.clone())));
    state.lsp.definition_provider =
        Some(Rc::new(RmlDefinitionProvider::new(client.clone(), uri.clone())));
    if let Some(legend) = client.semantic_tokens_legend() {
        state.lsp.semantic_tokens_provider =
            Some(Rc::new(RmlSemanticTokensProvider::new(client, uri, legend)));
    }
}
```

### Step 6: 创建 prelude.rs

```rust
//! 便捷 re-exports：`use rust_rml_client::prelude::*;`

pub use crate::editor::install_lsp_providers;
pub use crate::grammar::{language, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};
pub use crate::lsp_client::{file_path_to_uri, LspClient};
pub use crate::providers::{
    RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider, RmlSemanticTokensProvider,
};
pub use crate::registry::register_rml_language;
```

### Step 7: 更新 crates/rml/Cargo.toml

```toml
[package]
name = "rust-rml-client"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "RML client: tree-sitter grammar + LSP client + CodeEditor providers"

build = "build.rs"

[dependencies]
gpui-component = { workspace = true }
tree-sitter = "0.26"
tree-sitter-language = "0.1"
lsp-types = { workspace = true }
lsp-server = { workspace = true }
crossbeam-channel = { workspace = true }
anyhow = { workspace = true }
serde_json = { workspace = true }
log = { workspace = true }
url = { workspace = true }
ropey = { workspace = true }

[build-dependencies]
cc = "1"

# 删除 [lib] path = "lib.rs"，使用默认 src/lib.rs
```

（确认 `gpui-component`、`lsp-types`、`lsp-server`、`crossbeam-channel`、`anyhow`、`serde_json`、`log`、`url`、`ropey` 在 workspace.dependencies 中已有对应条目；若缺则按 demo/Cargo.toml 中的版本补齐到 workspace.dependencies。）

### Step 8: 更新根 Cargo.toml

`[workspace.members]` 中 `"crates/tree-sitter-rml"` → `"crates/rml"`
`[workspace.dependencies]` 中 `tree-sitter-rml = { path = "crates/tree-sitter-rml" }` → `rust-rml-client = { path = "crates/rml" }`

### Step 9: 更新 demo/Cargo.toml

- `tree-sitter-rml = { workspace = true }` → `rust-rml-client = { workspace = true }`
- `tree-sitter = "0.26"` → 删除（仅 registry.rs 用，已移入 rust-rml-client）
- `lsp-types`、`lsp-server`、`crossbeam-channel`、`url` → 检查 demo 自身是否还直接使用；若仅被已移走的文件使用则删除
- 保留 `gpui-component`、`anyhow`、`serde_json`、`log`、`ropey`（demo 其他模块仍用）

### Step 10: 更新 demo/src/app.rs

```rust
// 删除：
// use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
// use tree_sitter_rml::{language, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        cx.set_style("styles.css");
        cx.set_i18n("zh-CN");
        cx.set_theme("light");
        rust_rml_client::register_rml_language();
    }
}
```

### Step 11: 删除 demo 中已移走的文件

- `demo/src/lsp/lsp_client.rs`
- `demo/src/lsp/completion_provider.rs`
- `demo/src/lsp/hover_provider.rs`
- `demo/src/lsp/definition_provider.rs`
- `demo/src/lsp/semantic_tokens_provider.rs`

### Step 12: 更新 demo/src/lsp/mod.rs

删除已移走模块的 `pub mod` 声明，改为从 `rust_rml_client` re-export demo 仍需的类型：

```rust
pub mod code_editor_tab;
pub mod file_tree;
pub mod lsp_status;
#[path = "lsp_explorer_panel.rml.rs"]
pub mod lsp_explorer_panel;

// demo 仍需引用 LspClient 与 file_path_to_uri（CodeEditorTab 用）
pub use rust_rml_client::{file_path_to_uri, LspClient};
pub use code_editor_tab::CodeEditorTab;
pub use lsp_status::{ensure_lsp_status_item_registered, LspStatusState, LspStatusStateRef};
```

### Step 13: 简化 demo/src/lsp/code_editor_tab.rml.rs

将原 4 段 provider 安装代码替换为一行 `install_lsp_providers(...)`：

```rust
use rust_rml_client::{file_path_to_uri, install_lsp_providers, LspClient};
use crate::lsp::LspStatusStateRef;

// ...

let editor_state = cx.new(|cx| {
    let mut state = InputState::new(window, cx)
        .code_editor(language)
        .multi_line(true)
        .tab_size(TabSize { tab_size: 4, ..Default::default() })
        .default_value(&text);
    install_lsp_providers(&mut state, lsp_client.clone(), uri.clone());
    state
});
```

删除原 `use crate::lsp::{RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider, RmlSemanticTokensProvider};` 等导入。

### Step 14: 编译验证

```
cargo check -p rust-rml-client
cargo check -p rust-rml-demo
cargo test -p rust-rml-client --lib grammar   # 验证 tree-sitter 测试仍通过
```

## Assumptions & Decisions

1. **C 符号 `tree_sitter_rml()` 不重命名**：parser.c 中 C 函数名由 tree-sitter CLI 生成，与 Rust 包名无关。Rust 侧 extern 声明保持 `fn tree_sitter_rml() -> *const ()`，仅 `LanguageFn::from_raw(tree_sitter_rml)` 包装。包名改为 `rust-rml-client` 不影响 C 符号。

2. **`include_str!` 路径用 `CARGO_MANIFEST_DIR` 锚定**：`grammar.rs` 移入 `src/` 后，相对路径 `queries/highlights.scm` 会找不到。改用 `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/queries/highlights.scm"))` 锚定到 crate 根，保持 `queries/` 目录在根位置不变。

3. **`install_lsp_providers` 取 `&mut InputState` 而非 `&mut Lsp`**：`InputState` 是 demo 侧已拥有的类型，传 `&mut state` 比 `&mut state.lsp` 更自然且向后兼容（若未来 `Lsp` 字段重命名，本 API 不变）。

4. **`register_rml_language()` 无参数**：内部用 `LanguageRegistry::singleton()` 取全局实例，与现有 `Startup::on_launch` 行为一致。

5. **`prelude` 模块提供**：方便 `use rust_rml_client::prelude::*;` 一行集成。不强制使用。

6. **demo 仍保留 `pub use rust_rml_client::{LspClient, file_path_to_uri}`**：因为 `CodeEditorTab::new` 签名需要 `Arc<LspClient>` 参数，demo 内部其他模块（如 lsp_explorer_panel 创建 tab 时）仍需引用。

7. **不动 `crates/lsp`（LSP server）**：本次仅重构客户端。server 端在 workspace `exclude` 中，独立编译，不受影响。

8. **不引入 DAP 模块**：本次仅迁移现有 LSP 客户端逻辑到新 crate。DAP 客户端待后续开发时新增 `crates/rml/src/dap/` 子模块即可，crate 结构已为其预留。

9. **依赖版本对齐 workspace**：若 `lsp-server`、`url` 等尚未在 `workspace.dependencies` 中，先补到 workspace 再在 crate 内 `{ workspace = true }` 引用，避免版本漂移。

## Verification

1. **Step 14 后**：
   - `cargo check -p rust-rml-client` 无 error
   - `cargo check -p rust-rml-demo` 无 error
   - `cargo test -p rust-rml-client --lib grammar` 5 个 tree-sitter 测试全通过

2. **手动验证（用户后续）**：
   - 启动 demo → 打开 LSP Explorer 中 `.rml` 文件
   - 静态着色（tree-sitter）+ 动态着色（LSP semantic tokens）均正常
   - completion / hover / definition / formatting / rename / references / documentSymbol 命令均可用

## Execution Order

```
Step 1  (重命名目录)
  → Step 2  (重构 crate 内文件布局：创建 src/lib.rs + src/grammar.rs)
  → Step 3  (移入 lsp_client.rs + 4 个 providers)
  → Step 4  (创建 registry.rs)
  → Step 5  (创建 editor.rs)
  → Step 6  (创建 prelude.rs)
  → Step 7  (更新 crates/rml/Cargo.toml)
  → Step 8  (更新根 Cargo.toml)
  → Step 9  (更新 demo/Cargo.toml)
  → Step 10 (更新 demo/src/app.rs)
  → Step 11 (删除 demo 中已移走的 5 个文件)
  → Step 12 (更新 demo/src/lsp/mod.rs)
  → Step 13 (简化 code_editor_tab.rml.rs)
  → Step 14 (编译 + 测试验证)
```

严格串行。Step 1 是 Step 2-7 的前置。Step 7-8 是 Step 9 的前置。Step 9-12 是 Step 13 的前置。Step 13 是 Step 14 的前置。

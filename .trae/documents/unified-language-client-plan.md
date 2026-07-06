# LanguageClient 一体化架构修订 + 编译修复方案

## 背景

用户澄清：**RML 框架的 LSP/DAP 体系是 rust+rml 一体化设计**，`crates\lsp` 通过直接引入 rust-analyzer 源码定制开发，已集成两种语言支持。当前 `LanguageProfile::rml()` + `LanguageProfile::rust()` 分离预设是冗余接口设计，需统一。

同时，前一会话遗留两个问题：
1. demo 编译错误（`rml::LanguageClient` 找不到）
2. 批量替换 `\brml::` → `rust_rml_client::` 误伤了本应指向 `rust_rml_engine` 的宏引用

## 根因分析

### 编译错误根因
[main.rs:2](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs#L2) 的 `extern crate rust_rml_engine as rml;` 把 `rml` 别名指向 `rust_rml_engine`，**覆盖了** [demo/Cargo.toml:13](file:///e:/GitCode/RF/rust-gpui-rml/demo/Cargo.toml#L13) 的 Cargo 别名 `rml = { workspace = true }`（指向 `rust-rml-client`）。

因此 `use rml::LanguageClient` 实际在 `rust_rml_engine` 中查找 → 找不到。

### 批量替换副作用
[main.rs:15](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs#L15) 的 `#[rust_rml_client::main]` 错误 —— `main` 宏定义在 [crates/engine/src/lib.rs:21](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/lib.rs#L21)（`pub use rml_macros::main;`），应是 `#[rust_rml_engine::main]`。同理 `embed_assets!` 宏也定义在 engine crate（[lib.rs:72](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/lib.rs#L72)）。

---

## Part A：架构修订 —— rust+rml 一体化

### A1. 重构 `LanguageProfile`

**文件**：[crates/rml/src/language_profile.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/rml/src/language_profile.rs)

**变更**：
- 移除 `LanguageProfile` 的单值字段 `language_id` / `file_extensions` / `grammar`
- 新增 `languages: Vec<LanguageDescriptor>` 字段（描述该 server 处理的所有语言）
- 新增 `LanguageDescriptor` 结构体：
  ```rust
  pub struct LanguageDescriptor {
      pub language_id: SharedString,       // "rust" / "rml"
      pub file_extensions: Vec<SharedString>,
      pub grammar: Option<TreeSitterGrammar>,
  }
  ```
- 移除 `LanguageProfile::rml()` 和 `LanguageProfile::rust()`
- 新增 `LanguageProfile::unified()`：描述 `crates\lsp` 定制 rust-analyzer，包含 rust + rml 两个 `LanguageDescriptor`
  - rust: `grammar: None`（gpui-component 内置）
  - rml: `grammar: Some(TreeSitterGrammar { ... })`（crates/rml 自带）

### A2. 重构 `LanguageClient`

**文件**：[crates/rml/src/language_client.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/rml/src/language_client.rs)

**变更**：
- `new(profile, workspace_root)`：遍历 `profile.languages` 注册所有 grammar，spawn 一个 LSP server
- `open_document(uri, text)`：新增 `detect_language(uri)` 私有方法，从 URI 扩展名推断 `language_id`，替代原先的 `self.profile.language_id`
- 移除 `rml()` 和 `rust()` 便捷构造
- 新增 `unified(workspace_root)` 便捷构造 —— 等价于 `Self::new(LanguageProfile::unified(), workspace_root)`

### A3. 重构 `DebugProfile` 和 `DebugClient`

**文件**：[crates/rml/src/language_profile.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/rml/src/language_profile.rs) + [crates/rml/src/debug_client.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/rml/src/debug_client.rs)

**变更**：
- `DebugProfile`：移除 `language_id` 字段（DAP 协议无 language_id 概念，lldb-vscode 同时处理 rust+rml）
- 移除 `DebugProfile::rust()`，新增 `DebugProfile::unified()`
- `DebugClient`：移除 `rust()`，新增 `unified()`

### A4. 更新 `lib.rs` / `prelude.rs` 导出

**文件**：[crates/rml/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/rml/src/lib.rs) + [crates/rml/src/prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/rml/src/prelude.rs)

**变更**：新增 `LanguageDescriptor` 导出。

---

## Part B：编译修复

### B1. 修复 `main.rs` 宏引用

**文件**：[demo/src/main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs)

**变更**：
- 第 12 行注释：`rust_rml_client::main` → `rust_rml_engine::main`，`rust_rml_client::embed_assets!()` → `rust_rml_engine::embed_assets!()`
- 第 15 行：`#[rust_rml_client::main]` → `#[rust_rml_engine::main]`
- 第 2 行：移除 `extern crate rust_rml_engine as rml;`（已废弃，`rust_rml_engine::` 直接可用）

### B2. 更新 `demo/Cargo.toml`

**文件**：[demo/Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/demo/Cargo.toml)

**变更**：第 13 行 `rml = { workspace = true }` → `rust-rml-client = { workspace = true }`

### B3. 清理 workspace `Cargo.toml`

**文件**：[Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/Cargo.toml)

**变更**：移除第 31-32 行的 `rml` 别名：
```toml
# 删除：
# rml = { path = "crates/rml", package = "rust-rml-client" }
```
保留第 30 行 `rust-rml-client = { path = "crates/rml", package = "rust-rml-client" }`。

### B4. 更新 demo 调用点

**文件**：[demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

**变更**：第 141 行 `LanguageClient::rml(&workspace_root)` → `LanguageClient::unified(&workspace_root)`

### B5. 更新 demo 注释

**文件**：[demo/src/app.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/app.rs)

**变更**：第 3 行注释 `LanguageClient::rml()` → `LanguageClient::unified()`

---

## Part C：验证

1. `cargo check -p rust-rml-client` —— 客户端 crate 编译通过
2. `cargo check -p rust-rml-demo` —— demo crate 编译通过
3. `cargo test -p rust-rml-client` —— grammar 单元测试通过

---

## 假设与决策

1. **`crates\lsp` server 二进制名保持 `rml-lsp`**：这是定制 rust-analyzer 的产出二进制名，`LanguageProfile::unified()` 沿用。
2. **demo 中 `code_editor_tab.rml.rs` 的本地 `language` 变量保留**：该变量用于 `InputState::code_editor(language)`（tree-sitter 静态着色模式选择），是 demo UI 层关注点，与 LSP `language_id` 推断分离。`LanguageClient::open_document()` 内部独立从 URI 推断 LSP language_id。
3. **移除 `rml` workspace 别名**：用户记忆明确"不容忍兼容性设计"，别名导致 `extern crate` 冲突且无实际价值，直接使用 `rust-rml-client` 全名。
4. **`DebugProfile` 移除 `language_id`**：DAP 协议本身无 language_id 概念，lldb-vscode 同时调试 rust 和 rml（RML 编译到 Rust），一个 adapter 处理所有。
5. **`LanguageClient::new(profile, workspace_root)` 通用构造保留**：满足"极致易用"偏好（`unified()` 便捷 + `new()` 通用），不强制只能用预设。

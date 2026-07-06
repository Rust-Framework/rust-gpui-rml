# LanguageClient 封装 — 验证与收尾计划

## 背景

前一会话已完成 `rust-rml-client` 中 `LanguageClient` 封装的全部代码改造(原 15 步计划的 Step 1-14),但在最后一步验证编译时上下文丢失。本计划聚焦于**验证已完成的实现**并修复任何遗留编译错误。

代码审查已确认以下文件状态正确:
- `crates/rml/src/lib.rs` — 导出 `LanguageClient` / `DebugClient` / `LanguageProfile` / 4 个 `Lsp*Provider`
- `crates/rml/src/language_client.rs` — `LanguageClient` 结构体 + `rml()` / `rust()` 便捷构造 + `install_providers()`
- `crates/rml/src/language_profile.rs` — `LanguageProfile::rml()` / `rust()` + `DebugProfile::rust()`
- `crates/rml/src/debug_client.rs` — `DebugClient` 骨架(`todo!()`)
- `crates/rml/src/lsp_client.rs` — `spawn()` 接受 `&LanguageProfile`,profile 驱动二进制解析
- `crates/rml/src/providers/*.rs` — 4 个 `Lsp*Provider`(从 `Rml*Provider` 重命名)
- `crates/rml/Cargo.toml` — 已添加 `gpui = { workspace = true }`
- 工作区 `Cargo.toml` — 已添加 `rml = { path = "crates/rml", package = "rust-rml-client" }` 别名
- `demo/Cargo.toml` — `tree-sitter-rml` → `rml`,移除冗余依赖
- `demo/src/app.rs` — 移除手动 grammar 注册
- `demo/src/lsp/mod.rs` — 添加 `#[path]` 属性
- `demo/src/lsp/code_editor_tab.rml.rs` — `lsp_client` → `language_client`,使用 `install_providers()`
- `demo/src/shell/main_window.rml.rs` — `LanguageClient::rml()` + `language_client` 字段
- `demo/src/shell/workbench.rs` — `language_client` 字段贯穿 `LspWorkbench` / `LspWorkbenchProvider`

Grep 验证: `demo/src` 中已无 `lsp_client` / `Rml.*Provider` / `tree_sitter_rml` 残留引用。

## 当前任务

仅剩**验证步骤**未执行 — 需运行 `cargo check` 确认编译通过,并运行 grammar 测试。

## 执行步骤

### Step 1: 编译验证 rust-rml-client crate

```powershell
cargo check -p rust-rml-client 2>&1 | Select-Object -Last 60
```

**预期**: 编译通过,可能有少量 warning(死代码等)。

**验证标准**: 无 `error[E*]` 行。

### Step 2: 编译验证 rust-rml-demo

```powershell
cargo check -p rust-rml-demo 2>&1 | Select-Object -Last 80
```

**预期**: 编译通过。前一会话已修复 7 个错误:
- `demo/src/lsp/mod.rs` 添加 `#[path = "code_editor_tab.rml.rs"]`
- `demo/src/shell/workbench.rs` 添加 `use rml::LanguageClient;`
- `demo/src/shell/main_window.rml.rs` 添加 `use rml::LanguageClient;`

**验证标准**: 无 `error[E*]` 行。若出现新错误,逐个修复(可能涉及 `use` 导入或方法调用调整)。

### Step 3: 运行 grammar 单元测试

```powershell
cargo test -p rust-rml-client 2>&1 | Select-Object -Last 40
```

**预期**: tree-sitter grammar 解析测试通过(原 `tree-sitter-rml` 的测试套件)。

**验证标准**: `test result: ok. N passed; 0 failed`。

### Step 4: 错误修复(条件执行)

仅在 Step 1-3 出现错误时执行。可能的修复方向:
- 缺失 `use` 导入 → 添加对应 `use rml::...;`
- 方法签名不匹配 → 调整调用方参数
- 类型不匹配 → 检查 `Arc<LanguageClient>` vs `LanguageClient` 转换

### Step 5: 运行时验证说明(不自动执行)

实现正确性最终需运行时验证,但这超出本计划范围(需用户手动启动 demo):
- 启动 demo → 打开 LSP Explorer → 打开 `.rml` 文件
- 验证 tree-sitter 静态着色(tag/keyword/string 高亮)
- 验证 LSP 动态 semantic tokens(100ms debounce 后字段/类型着色)
- 验证补全/hover/跳转响应

## 设计决策记录(已在前一会话确定)

1. **一个 LanguageClient 实例服务一种语言** — 多语言场景创建多个实例
2. **`LanguageProfile::rust()` 的 `grammar: None`** — 依赖 gpui-component `tree-sitter-languages` feature 自动注册
3. **`LspClient` 仍为 pub** — 作为 `LanguageClient` 内部 IPC 层,但外部应优先使用 `LanguageClient`;`lsp()` 访问器暴露底层方法(formatting/rename/references)
4. **`DebugClient` 为骨架** — `todo!()` 占位,后续实现 DAP 协议
5. **Provider 命名从 `Rml*` 改为 `Lsp*`** — 语言无关,无 RML 特定逻辑
6. **工作区依赖别名 `rml`** — demo 侧短名引用 `use rml::LanguageClient;`

## 验证清单

- [ ] `cargo check -p rust-rml-client` 通过
- [ ] `cargo check -p rust-rml-demo` 通过
- [ ] `cargo test -p rust-rml-client` 通过
- [ ] (可选) demo 运行时 LSP 功能正常

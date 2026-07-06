# RML 语法服务收尾实现计划（Phase 3 收尾 + Phase 4）

## Summary

延续 `rml-syntax-highlighting-plan.md`（4 阶段总计划）与 `rml-syntax-highlighting-continuation.md`（上轮 continuation）。本轮聚焦两件事：

1. **完成 LSP 动态语义 tokens 层**（Phase 3 剩余 3.5–3.9）：修复 `binder.rs` 编译错误 → 声明 server capability → 新增 handler + dispatch 路由 → LSP crate 编译/测试通过。
2. **Demo 集成 gpui-component CodeEditor 双层着色**（Phase 4 全部 4.1–4.5）：`LspClient` 暴露 `semantic_tokens_full` + legend 缓存 → 实现 `DocumentRangeSemanticTokensProvider` → `CodeEditorTab` 安装 provider → demo 编译通过。

完成后 RML 编辑器具备：tree-sitter 静态层即时着色（Phase 2 已生效）+ LSP 动态层 100ms debounce 后增量着色（已解析字段=VARIABLE/DEFINITION、未解析=PROPERTY/DEPRECATED、组件标签=TYPE、事件处理器=FUNCTION 等）。

## Current State Analysis

### ✅ Phase 1 — Engine AST Directive spans
全部 `Directive` 变体已含 `span: Span`（`crates/engine/src/parser/ast.rs:85-109`）。

### ✅ Phase 2 — tree-sitter-rml 静态层
- `crates/tree-sitter-rml/lib.rs` 导出 `language()` + `HIGHLIGHTS_QUERY` + `INJECTIONS_QUERY`
- 根 `Cargo.toml` members 含 `crates/tree-sitter-rml`，workspace deps 含 `tree-sitter-rml`
- `demo/Cargo.toml` 依赖 `tree-sitter-rml` + `tree-sitter = "0.26"`
- `demo/src/app.rs` `Startup::on_launch` 注册 `"rml"` 到 `LanguageRegistry`（静态着色已生效）

### 🔄 Phase 3 — LSP 动态语义 tokens（部分完成 + 编译阻塞）
**已完成**：
- `crates/lsp/src/semantics/tokens.rs` — 9 个 token type + 4 个 modifier + `SpannedSemanticToken`
- `crates/lsp/src/semantics/mod.rs` — `pub mod tokens;`
- `crates/lsp/src/semantics/model.rs` — `SemanticModel` 含 `tokens: Vec<SpannedSemanticToken>`，`analyze_with_uri` 调 `binder::bind` 填充 `BindingResult`
- `crates/lsp/src/semantics/binder.rs` — `bind()` 返回 `BindingResult`，对所有 AST 构造发射 token；10 个单元测试

**编译阻塞 Bug**（必须先修）：
- `binder.rs` 第 151、164、174 行调用 `find_ident_span_in(span, source, ident)`，但该函数从未定义；现有同名工具为 `find_ident_in(span, source, ident)`（第 442 行）。三者函数体本应一致，仅需统一名称。不修此 bug，整个 LSP crate 无法编译。

**未完成**：
- `crates/lsp/src/server/connection.rs::build_capabilities` — 无 `semantic_tokens_provider` capability
- `crates/lsp/src/handlers/semantic_tokens.rs` — 不存在
- `crates/lsp/src/handlers/mod.rs` — 无 `pub mod semantic_tokens;`
- `crates/lsp/src/server/dispatch.rs::handle_request` — 无 `textDocument/semanticTokens/full` 与 `/range` 路由

### ⬜ Phase 4 — Demo CodeEditor 动态层集成
- `demo/src/lsp/lsp_client.rs` `LspClient` 无 `semantic_tokens_full` 方法；`initialize` 仅 `log::info!` 响应，未缓存 legend
- `demo/src/lsp/semantic_tokens_provider.rs` — 不存在
- `demo/src/lsp/mod.rs` — 无 `pub mod semantic_tokens_provider;`
- `demo/src/lsp/code_editor_tab.rml.rs::CodeEditorTab::new` — 仅安装 completion/hover/definition 三个 provider，未安装 `semantic_tokens_provider`

### 关键已验证 API
- gpui-component `Lsp` 结构体已有字段：`pub semantic_tokens_provider: Option<Rc<dyn DocumentRangeSemanticTokensProvider>>`（`crates/ui/src/input/lsp/mod.rs:37`）
- `Lsp::update_semantic_tokens` 已实现 100ms debounce + delta 解码 + viewport 过滤
- `DocumentRangeSemanticTokensProvider` trait：`fn legend() -> SemanticTokensLegend` + `fn semantic_tokens(&self, text: &Rope, range: Range<usize>, window, cx) -> Task<Result<SemanticTokens>>`
- `crates/lsp/src/server/conv.rs::span_to_range(span, source, line_starts) -> Range` — Span→LSP Range 转换
- `Document` 持有 `tree: Arc<SyntaxTree>`（含 `source: Arc<str>` + `line_starts: Vec<u32>`）+ `semantic: Arc<SemanticModel>`
- `Workspace::document(&uri) -> Option<&Document>`
- `crates/lsp` 在 workspace `exclude` 列表中 — Phase 3 验证用 `cd crates/lsp && cargo check`

---

## Proposed Changes

### Step 1: 修复 `binder.rs` 编译 bug

**文件**：`crates/lsp/src/semantics/binder.rs`

**变更**：第 151、164、174 行的 `find_ident_span_in` 改为 `find_ident_in`（与第 442 行定义一致）。

**Why**：函数名拼写不一致导致 LSP crate 无法编译，所有后续工作被阻塞。三处调用语义与 `find_ident_in` 完全相同（在 span 内查找标识符子串，返回精确子 span），仅名称错误。

**验证**：`cd crates/lsp && cargo check` 通过（暂不含新 handler，仅修 bug）。

---

### Step 2: `connection.rs::build_capabilities` 声明 semantic_tokens_provider

**文件**：`crates/lsp/src/server/connection.rs`（第 129–157 行）

**变更**：在 `ServerCapabilities { ... }` 内新增 `semantic_tokens_provider` 字段：
```rust
semantic_tokens_provider: Some(
    lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
        lsp_types::SemanticTokensOptions {
            legend: lsp_types::SemanticTokensLegend {
                token_types: crate::semantics::tokens::RML_TOKEN_TYPES.to_vec(),
                token_modifiers: crate::semantics::tokens::RML_TOKEN_MODIFIERS.to_vec(),
            },
            range: Some(true),
            full: Some(lsp_types::SemanticTokenFullOptions::Bool(true)),
            ..Default::default()
        },
    ),
),
```

**Why**：声明 server 同时支持 `/full` 与 `/range`，使 demo 客户端可任选其一（gpui-component 默认调 `/range`）。legend 引用 `semantics::tokens` 单一信源，避免重复定义导致漂移。

---

### Step 3: 新建 `handlers/semantic_tokens.rs`

**文件**：`crates/lsp/src/handlers/semantic_tokens.rs`（新建）

**职责**：将 `Document.semantic.tokens`（`Vec<SpannedSemanticToken>`）转 LSP `SemanticTokens { data: Vec<SemanticToken> }`（delta 编码）。

**公开 API**：
```rust
pub fn handle_full(params: serde_json::Value, state: &mut ServerState) -> anyhow::Result<Option<lsp_types::SemanticTokens>>;
pub fn handle_range(params: serde_json::Value, state: &mut ServerState) -> anyhow::Result<Option<lsp_types::SemanticTokens>>;
```

**实现步骤（两个函数共享核心逻辑）**：
1. 从 params 反序列化 `SemanticTokensParams`（full）或 `SemanticTokensRangeParams`（range），提取 `text_document.uri`
2. `state.workspace.document(&uri)` 取 `Document`；若不存在返回 `Ok(None)`
3. 取 `doc.tree.source`（`Arc<str>` → `&str`）、`doc.tree.line_starts`（`&[u32]`）、`doc.semantic.tokens`（`&[SpannedSemanticToken]`）
4. **range 分支**：从 `params.range` 取 LSP `Range`，反推字节区间 `[start_byte, end_byte)`，过滤 `token.span` 与之相交的 token
5. 将每个 `SpannedSemanticToken` 经 `conv::span_to_range(token.span, source, line_starts)` 转 LSP `Range`
6. **按 `range.start` 排序**（LSP delta 编码要求单调递增）
7. 计算 `length = span.len() as u32`（token 字节长度，UTF-8 字符数与字节长度在 RML 标识符/字符串场景一致；若含多字节字符，按 LSP 规范应用 UTF-16 长度 — 见下方 Decisions §1）
8. Delta 编码：`delta_line = cur.start.line - prev.start.line`；当同行 `delta_start = cur.start.character - prev.start.character`，否则 `delta_start = cur.start.character`
9. 返回 `SemanticTokens { data }`

**Why**：LSP `textDocument/semanticTokens` 协议要求 delta 编码的 `Vec<SemanticToken>`。`SpannedSemanticToken` 是绝对字节 span，需要转 LSP 位置 + delta 编码两步。复用 `conv::span_to_range` 避免重复实现位置转换。

---

### Step 4: `handlers/mod.rs` 加 `pub mod semantic_tokens;`

**文件**：`crates/lsp/src/handlers/mod.rs`

**变更**：在现有 `pub mod ...` 列表中新增一行：
```rust
pub mod semantic_tokens;
```

**Why**：模块声明，使 `dispatch.rs` 可通过 `handlers::semantic_tokens::handle_full` 引用。

---

### Step 5: `dispatch.rs::handle_request` 加路由

**文件**：`crates/lsp/src/server/dispatch.rs`（在 `rename` 分支后、`_` 分支前）

**变更**：新增两个 match 分支：
```rust
"textDocument/semanticTokens/full" => {
    handlers::semantic_tokens::handle_full(req.params, state)
        .map(|v| v.and_then(|t| serde_json::to_value(t).ok()))
}
"textDocument/semanticTokens/range" => {
    handlers::semantic_tokens::handle_range(req.params, state)
        .map(|v| v.and_then(|t| serde_json::to_value(t).ok()))
}
```

**Why**：将 LSP method 字符串路由到新 handler。返回 `Option<SemanticTokens>` 序列化为 JSON 响应，与现有 handler 风格一致。

---

### Step 6: LSP crate 编译 + 测试验证

**命令**：
```
cd crates/lsp && cargo check
cd crates/lsp && cargo test --lib semantics
```

**验证标准**：
- `cargo check` 无错误（仅允许 pre-existing warnings）
- `binder.rs` 内 10 个单元测试全部通过
- `tokens.rs` 模块加载无问题

**Why**：Phase 3 完成的硬性出口标准。LSP crate 在 workspace `exclude` 中，必须独立验证。

---

### Step 7: `LspClient` 加 `semantic_tokens_full` + legend 缓存

**文件**：`demo/src/lsp/lsp_client.rs`

**变更**：
1. `LspClient` 结构体新增字段：
   ```rust
   semantic_tokens_legend: std::sync::Mutex<Option<lsp_types::SemanticTokensLegend>>,
   ```
2. `spawn` 中构造 `Self` 时初始化为 `Mutex::new(None)`
3. `initialize`（第 159–177 行）在收到响应后解析 legend 并缓存：
   ```rust
   let result = rx.recv()??;
   let legend = result.get("capabilities")
       .and_then(|c| c.get("semanticTokensProvider"))
       .and_then(|p| p.get("legend"))
       .and_then(|l| serde_json::from_value::<lsp_types::SemanticTokensLegend>(l.clone()).ok());
   if let Some(lg) = legend {
       *self.semantic_tokens_legend.lock().unwrap() = Some(lg);
   }
   ```
4. 新增公开方法：
   ```rust
   pub fn semantic_tokens_legend(&self) -> Option<lsp_types::SemanticTokensLegend> {
       self.semantic_tokens_legend.lock().unwrap().clone()
   }

   pub fn semantic_tokens_full(&self, uri: &lsp_types::Uri) -> crossbeam_channel::Receiver<Result<serde_json::Value>> {
       let params = serde_json::json!({
           "textDocument": { "uri": uri.as_str() },
       });
       self.send_request("textDocument/semanticTokens/full", params)
   }
   ```

**Why**：gpui-component 的 `Lsp::update_semantic_tokens` 内部会调 `provider.semantic_tokens(...)`，provider 需通过 IPC 拉取 LSP 响应；legend 必须在 `initialize` 时缓存，因为 provider 实例化时 `Lsp` 已开始查询 legend。`send_request` 已是 `&self`，无需额外同步。

---

### Step 8: 新建 `demo/src/lsp/semantic_tokens_provider.rs`

**文件**：`demo/src/lsp/semantic_tokens_provider.rs`（新建）

**职责**：实现 gpui-component `DocumentRangeSemanticTokensProvider`，将 LSP IPC 响应转为 `SemanticTokens`。

**实现**：
```rust
use std::rc::Rc;
use std::sync::Arc;

use gpui::{App, Task, Window, app::Context};
use gpui_component::input::lsp::semantic_tokens::DocumentRangeSemanticTokensProvider;
use lsp_types::{SemanticTokens, SemanticTokensLegend, Uri};
use serde_json::Value;

use crate::lsp::LspClient;

pub struct RmlSemanticTokensProvider {
    client: Arc<LspClient>,
    uri: Uri,
    legend: SemanticTokensLegend,
}

impl RmlSemanticTokensProvider {
    pub fn new(client: Arc<LspClient>, uri: Uri, legend: SemanticTokensLegend) -> Self {
        Self { client, uri, legend }
    }
}

impl DocumentRangeSemanticTokensProvider for RmlSemanticTokensProvider {
    fn legend(&self) -> SemanticTokensLegend {
        self.legend.clone()
    }

    fn semantic_tokens(
        &self,
        _text: &gpui_component::editor::Rope,
        _range: std::ops::Range<usize>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<SemanticTokens>> {
        let rx = self.client.semantic_tokens_full(&self.uri);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()
                .map_err(|e| anyhow::anyhow!("channel closed: {e}"))??;
            let tokens: SemanticTokens = serde_json::from_value(resp)?;
            Ok(tokens)
        })
    }
}
```

**注意**：`Rope` 类型与 `gpui_component::input::lsp::semantic_tokens` 模块中 trait 签名一致 — 实施时以实际 trait 签名为准调整 import（gpui-component 当前 checkout `063e55b`，trait 用 `Rope` 类型，需从对应模块 re-export 引入）。

**Why**：gpui-component 的 `Lsp::update_semantic_tokens` 已实现 100ms debounce + viewport 过滤 + delta 解码，provider 只需在 trait 方法被调用时通过 IPC 拉取完整 `SemanticTokens` 返回。`/range` vs `/full` 选择：trait 接收 `range` 参数但 gpui-component 内部已做 viewport 过滤，provider 调 `/full` 即可（RML 文件小，全量 token 通常 < 1KB），简化实现。

---

### Step 9: `demo/src/lsp/mod.rs` 加模块导出

**文件**：`demo/src/lsp/mod.rs`

**变更**：新增：
```rust
pub mod semantic_tokens_provider;
pub use semantic_tokens_provider::RmlSemanticTokensProvider;
```

**Why**：使 `CodeEditorTab` 可通过 `crate::lsp::RmlSemanticTokensProvider` 引用。

---

### Step 10: `CodeEditorTab::new` 安装 semantic_tokens_provider

**文件**：`demo/src/lsp/code_editor_tab.rml.rs`（第 52–73 行 `cx.new` 闭包内）

**变更**：在现有三个 provider 安装后追加：
```rust
if let Some(legend) = lsp_client.semantic_tokens_legend() {
    state.lsp.semantic_tokens_provider = Some(std::rc::Rc::new(
        crate::lsp::RmlSemanticTokensProvider::new(
            lsp_client.clone(),
            uri.clone(),
            legend,
        ),
    ));
}
```

**Why**：legend 在 `LspClient::spawn` 的 `initialize` 阶段已缓存；此处仅在有 legend 时安装 provider，避免 `Lsp::update_semantic_tokens` 调用 trait 方法时 panic。provider 持有 `Arc<LspClient>` 与 `Uri` 副本，与现有 completion/hover/definition provider 模式一致。

---

### Step 11: Demo 编译验证

**命令**：
```
cargo check -p rust-rml-demo
```

**验证标准**：
- 编译通过，无 error（仅允许 pre-existing dead_code warnings）
- 无类型不匹配（特别关注 `DocumentRangeSemanticTokensProvider` trait 签名匹配）

**Why**：Phase 4 完成的硬性出口标准。运行时验证（启动 demo 打开 .rml 文件观察双层着色）由用户在后续手动完成。

---

## Assumptions & Decisions

1. **token length 单位**：LSP `SemanticToken.length` 按 UTF-16 code unit 计数（协议规范）。RML 标识符/属性名/字符串值绝大多数为 ASCII，字节长度 == UTF-16 长度。实施时若 `SpannedSemanticToken.span` 内含非 ASCII 字符（如中文属性值字符串），需用 `span_text.chars().map(|c| c.len_utf16() as u32).sum::<u32>()` 计算 length。**默认实施按 UTF-16 计算**以符合协议规范。

2. **`/full` vs `/range` 选择**：provider 实现选择 `/full`（返回完整文档 tokens），由 gpui-component 内部 `Lsp::semantic_tokens_for_range` 做 viewport 二分过滤。理由：RML 文件小（通常 < 10KB），全量 tokens 通常 < 1KB 序列化后；`/range` 需额外实现字节区间反推，增加复杂度无收益。

3. **legend 单一信源**：`RML_TOKEN_TYPES` / `RML_TOKEN_MODIFIERS` 定义在 `crates/lsp/src/semantics/tokens.rs`；server 通过 `build_capabilities` 引用；demo 通过 `initialize` 响应透传缓存；provider 通过 `LspClient::semantic_tokens_legend()` 读取。三方链路无硬编码 legend，避免漂移。

4. **`crates/lsp` workspace 排除**：根 `Cargo.toml` `exclude` 含 `crates/lsp`，Step 6 验证必须用 `cd crates/lsp && cargo check`，不可用 `cargo check -p`。

5. **不修改既有 handler 风格**：新增 `handlers/semantic_tokens.rs` 函数签名 `(params: Value, state: &mut ServerState) -> Result<Option<T>>` 与现有 handler 一致；`dispatch.rs` 路由模式与现有 8 个路由一致。

6. **不修改 `binder.rs` 业务逻辑**：Step 1 仅修复函数名拼写错误（`find_ident_span_in` → `find_ident_in`），不动 token 发射规则、不动诊断逻辑、不动测试。

7. **`DocumentRangeSemanticTokensProvider` trait 签名以实际源码为准**：Phase 1 探查报告的签名基于 `target/doc` HTML 反推。Step 8 实施时必须 `Read` `crates/ui/src/input/lsp/semantic_tokens.rs` 实际源码确认 `Rope` 类型来源、`Task` 返回类型、`Range` 类型，再调整 import。若 trait 签名与计划有偏差，以源码为准调整 `RmlSemanticTokensProvider::semantic_tokens` 签名。

---

## Verification Steps

1. **Step 1 后**：`cd crates/lsp && cargo check` 通过（确认 bug 修复）
2. **Step 6 后**：`cd crates/lsp && cargo check` 通过 + `cd crates/lsp && cargo test --lib semantics` 全部测试通过
3. **Step 11 后**：`cargo check -p rust-rml-demo` 通过
4. **手动验证（用户后续）**：启动 demo → 打开 LSP Explorer 中的 `.rml` 文件 → 观察
   - **静态层（即时）**：`<div>` 标签着色、`if` 指令为 keyword、字符串为 string
   - **动态层（100ms debounce 后）**：已解析字段为 variable（DEFINITION 绿色）、未解析字段为 property（DEPRECATED 警告色）、`<Button>` 为 type、`onclick={fn}` 为 function
   - **编辑代码**：`didChange` 触发 `update_semantic_tokens`，动态层刷新

---

## Execution Order

```
Step 1 (修 binder bug)
  → Step 2 (capabilities)
  → Step 3 (handler)
  → Step 4 (mod 声明)
  → Step 5 (dispatch 路由)
  → Step 6 (LSP 编译+测试验证)
  → Step 7 (LspClient)
  → Step 8 (provider)
  → Step 9 (demo mod 导出)
  → Step 10 (CodeEditorTab 安装)
  → Step 11 (demo 编译验证)
```

严格串行。Step 1 是 Step 2–6 的前置（不修无法编译）。Step 6 是 Step 7–11 的前置（LSP 不通则 demo 无响应）。Step 7 是 Step 8 的前置（provider 依赖 `LspClient::semantic_tokens_full`）。Step 8–10 是 Step 11 的前置。

# RML 语法服务：剩余实现计划（Phase 2 收尾 + Phase 3 + Phase 4）

## Summary

延续 `rml-syntax-highlighting-plan.md`（已批准的 4 阶段总计划）。Phase 1 已完成；Phase 2 crate 文件已创建但 workspace 集成未完成；Phase 3、4 未开始。本计划聚焦剩余工作，所有 API 已对照 gpui-component `f7e8717` 源码与项目当前状态验证。

## Current State Analysis

### Phase 1 ✅ COMPLETED
- `Directive` 枚举所有变体已含 `span: Span`（`crates/engine/src/parser/ast.rs:85-109`）
- 全部 match site 已更新（engine + LSP crate）
- `cargo test -p rust-rml-engine` 通过

### Phase 2 🔄 PARTIALLY DONE
**已完成**：crate 文件已创建
- `crates/tree-sitter-rml/grammar.js` — RML 语法定义
- `crates/tree-sitter-rml/src/parser.c` — tree-sitter generate 产物（check-in）
- `crates/tree-sitter-rml/Cargo.toml` — `tree-sitter = "0.26"` + `cc = "1"`
- `crates/tree-sitter-rml/build.rs` — cc 编译脚本
- `crates/tree-sitter-rml/lib.rs` — `LANGUAGE: LanguageFn` + `HIGHLIGHTS_QUERY` + `INJECTIONS_QUERY` + 5 单元测试
- `crates/tree-sitter-rml/queries/highlights.scm` — 高亮查询
- `crates/tree-sitter-rml/queries/injections.scm` — rust 注入查询

**未完成**（本计划覆盖）：
1. workspace 集成（root `Cargo.toml` members + dependencies）
2. demo 依赖添加
3. demo 启动注册 `LanguageRegistry`
4. 编译 + 测试验证

### Phase 3 ⬜ NOT STARTED
- `SemanticModel` 只有 `diagnostics` 字段（`crates/lsp/src/semantics/model.rs:13-15`）
- `binder.rs` 返回 `Vec<SemanticDiagnostic>`（无 tokens）
- `build_capabilities()` 无 `semantic_tokens_provider`（`crates/lsp/src/server/connection.rs:129-156`）
- `dispatch.rs` 无 semantic tokens 路由
- 无 `crates/lsp/src/handlers/semantic_tokens.rs`
- 无 `crates/lsp/src/semantics/tokens.rs`

### Phase 4 ⬜ NOT STARTED
- `LspClient` 无 `semantic_tokens_full` 方法（`demo/src/lsp/lsp_client.rs`）
- `LspClient::initialize` 仅 log 响应，未缓存 legend
- `CodeEditorTab::new` 未安装 `semantic_tokens_provider`
- 无 `demo/src/lsp/semantic_tokens_provider.rs`

## Verified API (gpui-component @ f7e8717)

### LanguageRegistry / LanguageConfig
位于 `crates/ui/src/highlighter/registry.rs`：
```rust
pub struct LanguageConfig {
    pub name: SharedString,
    pub language: tree_sitter::Language,
    pub injection_languages: Vec<SharedString>,
    pub highlights: SharedString,
    pub injections: SharedString,
    pub locals: SharedString,
}
impl LanguageConfig {
    pub fn new(name, language, injection_languages, highlights: &str, injections: &str, locals: &str) -> Self
}
pub struct LanguageRegistry { ... }
impl LanguageRegistry {
    pub fn singleton() -> &'static LazyLock<LanguageRegistry>
    pub fn register(&self, lang: &str, config: &LanguageConfig)
}
```

### DocumentRangeSemanticTokensProvider
位于 `crates/ui/src/input/lsp/semantic_tokens.rs:37-56`：
```rust
pub trait DocumentRangeSemanticTokensProvider {
    fn legend(&self) -> SemanticTokensLegend;
    fn semantic_tokens(&self, text: &Rope, range: Range<usize>, window: &mut Window, cx: &mut App) -> Task<Result<SemanticTokens>>;
}
```
- `Lsp` 结构体已有字段：`pub semantic_tokens_provider: Option<Rc<dyn DocumentRangeSemanticTokensProvider>>`（`crates/ui/src/input/lsp/mod.rs:37`）
- `Lsp::update_semantic_tokens` 已实现 100ms debounce + delta 解码 + viewport 过滤，provider 只需返回 `SemanticTokens`

### 项目内现有基础设施
- `Document` 持有 `tree: Arc<SyntaxTree>`（含 `source: Arc<str>` + `line_starts: Vec<u32>`）+ `semantic: Arc<SemanticModel>`
- `conv::span_to_range(span, source, line_starts)` — Span→Range 转换
- `tags::is_builtin(tag: &str) -> bool` — HTML vs 组件标签判定
- `StructMetadata` 含 `observable_fields` / `computed_methods` / `commands` — 已解析 vs 未解析判定

---

## Proposed Changes

### Phase 2 收尾：workspace 集成 + demo 注册

#### 2.1 根 `Cargo.toml`（`e:\GitCode\RF\rust-gpui-rml\Cargo.toml`）
- `[workspace] members` 加 `"crates/tree-sitter-rml"`（当前：`["crates/core", "crates/macros", "crates/engine", "crates/ui", "crates/app", "demo"]`）
- `[workspace.dependencies]` 加 `tree-sitter-rml = { path = "crates/tree-sitter-rml" }`

**Why**：使 tree-sitter-rml 进入 workspace 编译图，供 demo 依赖。

#### 2.2 demo `Cargo.toml`（`e:\GitCode\RF\rust-gpui-rml\demo\Cargo.toml`）
- `[dependencies]` 加 `tree-sitter-rml = { workspace = true }`
- 加 `tree-sitter = "0.26"`（demo 直接用 `tree_sitter::Language::new` 构造 Language）

**Why**：demo 启动时需调用 `tree_sitter_rml::LANGUAGE` 和 `tree_sitter::Language::new`。

#### 2.3 demo 启动注册（`e:\GitCode\RF\rust-gpui-rml\demo\src\app.rs`）
在 `Startup::on_launch` 中注册 RML 语言到 `LanguageRegistry`：
```rust
use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
use tree_sitter_rml::{LANGUAGE, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};

fn on_launch(&mut self, cx: &mut App) {
    cx.set_style("styles.css");
    cx.set_i18n("zh-CN");
    cx.set_theme("light");

    // 注册 RML 语法到 gpui-component LanguageRegistry
    LanguageRegistry::singleton().register("rml", &LanguageConfig::new(
        "rml",
        tree_sitter::Language::new(LANGUAGE),
        vec!["rust".into()],
        HIGHLIGHTS_QUERY,
        INJECTIONS_QUERY,
        "",
    ));
}
```

**Why**：`InputState::code_editor("rml")` 在 `CodeEditorTab::new` 中被调用，但 "rml" 未注册 → 当前回退为纯文本。注册后 tree-sitter 静态层即时着色。

#### 2.4 验证
- `cargo build -p tree-sitter-rml` 通过
- `cargo test -p tree-sitter-rml` 5 个单元测试通过
- `cargo check -p rust-rml-demo` 通过
- demo 启动后 `CodeEditor` 中 `.rml` 文件有 tree-sitter 静态着色

---

### Phase 3：LSP 语义 Tokens（动态层）

#### 3.1 新文件 `crates/lsp/src/semantics/tokens.rs`
定义 token 类型常量 + `SpannedSemanticToken` 结构：
```rust
use lsp_types::{SemanticTokenModifier, SemanticTokenType};
use rust_rml_engine::parser::Span;

pub const RML_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,    // 0: 指令名 if/each/...
    SemanticTokenType::TAG,        // 1: HTML 标签名
    SemanticTokenType::TYPE,       // 2: 组件标签（PascalCase）
    SemanticTokenType::ATTRIBUTE,  // 3: 属性名
    SemanticTokenType::STRING,     // 4: 静态属性值
    SemanticTokenType::VARIABLE,   // 5: 已解析绑定字段
    SemanticTokenType::PROPERTY,   // 6: 未解析绑定字段
    SemanticTokenType::FUNCTION,   // 7: 事件处理器/命令
    SemanticTokenType::COMMENT,    // 8
];

pub const RML_TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,   // bit 0: ref 目标 / each 迭代变量
    SemanticTokenModifier::DEFINITION,    // bit 1: 已解析绑定
    SemanticTokenModifier::DEPRECATED,    // bit 2: 未解析绑定
    SemanticTokenModifier::MODIFICATION,  // bit 3: model 双向绑定
];

pub struct SpannedSemanticToken {
    pub span: Span,
    pub token_type: u32,
    pub token_modifiers: u32,
}
```

**Why**：legend 单一信源，server capabilities 声明 + demo 透传统一引用。

#### 3.2 `crates/lsp/src/semantics/mod.rs`
加 `pub mod tokens;`

#### 3.3 `crates/lsp/src/semantics/model.rs`
`SemanticModel` 加 `tokens` 字段：
```rust
pub struct SemanticModel {
    pub diagnostics: Vec<SemanticDiagnostic>,
    pub tokens: Vec<SpannedSemanticToken>,
}
```
`analyze_with_uri` 改为调用 `binder::bind` 返回的 `BindingResult`，填充 `tokens`。

#### 3.4 `crates/lsp/src/semantics/binder.rs`
- `bind()` 签名改为返回 `BindingResult { diagnostics, tokens }`
- `bind_element` / `bind_node` 在现有诊断逻辑基础上，同步发射 `SpannedSemanticToken`
- 发射规则（每个构造发射一个 token）：

| 构造 | token_type | modifiers | span 来源 |
|------|-----------|-----------|----------|
| 指令名（if/each/model/...） | KEYWORD | — | `Directive::* span` 内按指令名长度切分 |
| 指令绑定表达式根标识符（已解析） | VARIABLE | DEFINITION | `Directive::* span` 内扫描标识符 |
| 指令绑定表达式根标识符（未解析） | PROPERTY | DEPRECATED | 同上 |
| `model` 绑定字段 | VARIABLE | DEFINITION + MODIFICATION | `Directive::Model.span` 内 |
| `each` 迭代变量 | VARIABLE | DECLARATION | `Directive::Each.span` 内扫描 `item` |
| `each` 迭代源 | VARIABLE/PROPERTY | DEFINITION/DEPRECATED | `Directive::Each.span` 内扫描 `iterable` |
| 属性名 | ATTRIBUTE | — | `Attribute::* span` 内按 name 长度切分 |
| `Bind` 表达式根标识符 | VARIABLE/PROPERTY | DEFINITION/DEPRECATED | `Attribute::Bind.span` 内 |
| `Event` handler 名 | FUNCTION | DEFINITION(已注册)/DEPRECATED(未注册) | `Attribute::Event.span` 内 |
| 插值表达式根标识符 | VARIABLE/PROPERTY | DEFINITION/DEPRECATED | `Interpolation.span` 内 |
| 组件标签名（PascalCase） | TYPE | — | `Element.span` 内按 tag 长度 |
| HTML 标签名 | TAG | — | 同上 |

**子 span 提取策略**：`source[directive.span.start..directive.span.end]` 内用 `find` 定位指令名 + 标识符。失败时退化为整个 `directive.span`（发射但不够精确，不阻塞）。

**Why**：binder 已遍历 AST 做绑定检查，复用同一次遍历发射 tokens，零额外解析成本。

#### 3.5 `crates/lsp/src/server/connection.rs::build_capabilities`
加 `semantic_tokens_provider`：
```rust
semantic_tokens_provider: Some(
    lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
        lsp_types::SemanticTokensOptions {
            legend: lsp_types::SemanticTokensLegend {
                token_types: RML_TOKEN_TYPES.to_vec(),
                token_modifiers: RML_TOKEN_MODIFIERS.to_vec(),
            },
            range: Some(true),
            full: Some(lsp_types::SemanticTokenFullOptions::Bool(true)),
            ..Default::default()
        },
    ),
),
```

#### 3.6 新文件 `crates/lsp/src/handlers/semantic_tokens.rs`
```rust
pub fn handle_full(params: Value, state: &mut ServerState) -> Result<Option<SemanticTokens>>
pub fn handle_range(params: Value, state: &mut ServerState) -> Result<Option<SemanticTokens>>
```
实现步骤：
1. 从 params 提取 `uri`（`SemanticTokensParams` / `SemanticTokensRangeParams`）
2. `state.workspace.document(&uri)` 取 `Document`
3. 取 `doc.tree.source` + `doc.tree.line_starts` + `doc.semantic.tokens`
4. （`handle_range` 仅）过滤 range 外的 token
5. 用 `conv::span_to_range` 转 LSP `Range`
6. Delta 编码为 `Vec<SemanticToken>`：按 `range.start` 排序后计算 `delta_line`/`delta_start`/`length`/`token_type`/`token_modifiers_bitset`
7. 返回 `SemanticTokens { data }`

#### 3.7 `crates/lsp/src/handlers/mod.rs`
加 `pub mod semantic_tokens;`

#### 3.8 `crates/lsp/src/server/dispatch.rs::handle_request`
加两个路由：
```rust
"textDocument/semanticTokens/full" => handlers::semantic_tokens::handle_full(req.params, state)
    .map(|v| v.and_then(|t| serde_json::to_value(t).ok()))
"textDocument/semanticTokens/range" => handlers::semantic_tokens::handle_range(req.params, state)
    .map(|v| v.and_then(|t| serde_json::to_value(t).ok()))
```

#### 3.9 验证
- `cd crates/lsp && cargo check` 通过
- 新增测试 `semantic_tokens_full_returns_delta`：`<div if={count}>` 返回 `keyword`(if) + `variable`(count, DEFINITION)
- 新增测试 `semantic_tokens_range_filters`

---

### Phase 4：Demo 集成

#### 4.1 `demo/src/lsp/lsp_client.rs`
- `LspClient` 加字段 `semantic_tokens_legend: Arc<Mutex<Option<SemanticTokensLegend>>>`
- `initialize` 解析 `result.capabilities.semanticTokensProvider.legend` 并缓存
- 加 `pub fn semantic_tokens_legend(&self) -> Option<SemanticTokensLegend>`
- 加 `pub fn semantic_tokens_full(&self, uri: &Uri) -> Receiver<Result<Value>>`：
```rust
pub fn semantic_tokens_full(&self, uri: &Uri) -> Receiver<Result<Value>> {
    let params = serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
    });
    self.send_request("textDocument/semanticTokens/full", params)
}
```

**Why**：provider 需要 legend（从 initialize 缓存）+ IPC 通道拉取 tokens。

#### 4.2 新文件 `demo/src/lsp/semantic_tokens_provider.rs`
```rust
pub struct RmlSemanticTokensProvider {
    client: Arc<LspClient>,
    uri: Uri,
    legend: SemanticTokensLegend,
}

impl RmlSemanticTokensProvider {
    pub fn new(client: Arc<LspClient>, uri: Uri, legend: SemanticTokensLegend) -> Self { ... }
}

impl DocumentRangeSemanticTokensProvider for RmlSemanticTokensProvider {
    fn legend(&self) -> SemanticTokensLegend { self.legend.clone() }
    fn semantic_tokens(&self, _text: &Rope, _range: Range<usize>, _window: &mut Window, _cx: &mut App) -> Task<Result<SemanticTokens>> {
        let rx = self.client.semantic_tokens_full(&self.uri);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()??;
            let tokens: SemanticTokens = serde_json::from_value(resp)?;
            Ok(tokens)
        })
    }
}
```

**Why**：实现 gpui-component trait，将 LSP IPC 响应转为 `SemanticTokens`。`update_semantic_tokens` 已在 `Lsp` 内部实现 100ms debounce + delta 解码 + viewport 过滤。

#### 4.3 `demo/src/lsp/mod.rs`
加 `pub mod semantic_tokens_provider;` + `pub use semantic_tokens_provider::RmlSemanticTokensProvider;`

#### 4.4 `demo/src/lsp/code_editor_tab.rml.rs::CodeEditorTab::new`
在现有 providers（completion/hover/definition）后加：
```rust
if let Some(legend) = lsp_client.semantic_tokens_legend() {
    state.lsp.semantic_tokens_provider = Some(std::rc::Rc::new(
        RmlSemanticTokensProvider::new(lsp_client.clone(), uri.clone(), legend),
    ));
}
```

#### 4.5 验证
- `cargo check -p rust-rml-demo` 通过
- 启动 demo，打开 LSP Explorer 中的 `.rml` 文件
- **静态层**（即时）：`<div>` 标签着色、`if` 指令为 keyword、字符串为 string
- **动态层**（100ms debounce 后）：`count` 字段为 variable（DEFINITION），未解析字段为 property（DEPRECATED），`<Button>` 为 type，`onclick={fn}` 为 function
- 编辑代码后 `didChange` 触发 `update_semantic_tokens`，动态层刷新

---

## Assumptions & Decisions

1. **tree-sitter 版本对齐**：`Cargo.lock` 中 `tree-sitter = "0.26"`，tree-sitter-rml crate 已使用 `tree-sitter = "0.26"`，版本一致。
2. **`crates/lsp` 在 workspace exclude 中**：Phase 3 验证用 `cd crates/lsp && cargo check`，不阻塞主 workspace。
3. **legend 一致性**：`RML_TOKEN_TYPES`/`RML_TOKEN_MODIFIERS` 单一信源在 `semantics/tokens.rs`；server 通过 `build_capabilities` 声明，demo 通过 `initialize` 响应透传。
4. **Directive 子 span 精度**：指令名/表达式根标识符的子 span 在 binder emitter 内用源码扫描提取（`source[directive.span]` 内 `find` 指令名 + 标识符），失败时退化为整个 `directive.span`。
5. **`Number` token 类型移除**：原计划含 `NUMBER`(9)，但 RML 中数字字面量出现在绑定表达式内（由 rust injection 处理），LSP 不单独发射。最终 `RML_TOKEN_TYPES` 为 9 项（0-8）。
6. **demo 启动注册位置**：`Startup::on_launch` 是 `IAppLifecycle` 的启动钩子，是注册 `LanguageRegistry` 的正确位置（在窗口创建前）。

## Verification Steps

1. **Phase 2**：`cargo build -p tree-sitter-rml` + `cargo test -p tree-sitter-rml` + `cargo check -p rust-rml-demo`
2. **Phase 3**：`cd crates/lsp && cargo check` + `cd crates/lsp && cargo test`（新增 semantic_tokens 测试）
3. **Phase 4**：`cargo check -p rust-rml-demo` + 手动启动 demo 验证静态层 + 动态层着色

## Execution Order

```
Phase 2 收尾（workspace 集成） → Phase 3（LSP 语义） → Phase 4（Demo 集成）
```

Phase 3 依赖 Phase 1（已完成）。Phase 4 依赖 Phase 2 + 3。串行执行。

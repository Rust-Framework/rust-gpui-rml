# RML 专业语法服务：tree-sitter 静态层 + LSP 语义动态层

## Context

`crates/lsp` 当前仅提供补全/悬停/定义/引用/符号/格式化/签名/重命名，**无着色服务**。`SemanticModel` 只存诊断，`binder.rs` 解析绑定后丢弃结果。demo 的 `CodeEditor` 用 `code_editor("rml")` 但 "rml" 未注册到 `LanguageRegistry`，回退为纯文本。

本计划为 RML 架构提供专业语法服务：

1. **静态层**：独立 `tree-sitter-rml` grammar，注册到 gpui-component `LanguageRegistry`，提供即时结构化着色（标签/属性/字符串/注释/指令名）
2. **动态层**：LSP `textDocument/semanticTokens` 完整实现，扩展 `binder.rs` 返回 span→语义映射，区分已解析/未解析绑定、组件/HTML 标签、命令匹配状态
3. **demo 集成**：`CodeEditorTab` 安装 `DocumentRangeSemanticTokensProvider`，通过已有 `LspClient` IPC 拉取语义 tokens，与 tree-sitter 底层叠加

***

## Phase 1：Engine AST 扩展 — Directive 加 span

### 数据结构变更（`crates/engine/src/parser/ast.rs`）

将 `Directive` 各变体从 tuple 改为 struct 变体，附加 `span: Span`：

```rust
pub enum Directive {
    If { expr: String, span: Span },
    Else { span: Span },
    Each { clause: EachClause, span: Span },
    Key { expr: String, span: Span },
    Model { field: String, converter: Option<String>, span: Span },
    Show { expr: String, span: Span },
    Once { span: Span },
    Html { expr: String, span: Span },
    Ref { name: String, span: Span },
}
```

`EachClause` **不加子 span** — LSP token emitter 从 `Directive::Each.span`（覆盖 `each={item in items}`）内扫描源码提取 `item`/`in`/`iterable` 子区间，避免扩大 AST 改动面。

### 需修改文件

| 文件                                                                             | 改动                                                                                                |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| `crates/engine/src/parser/ast.rs`                                              | 上述结构变更                                                                                            |
| `crates/engine/src/parser/mod.rs`                                              | `build_element`（行 204-248）6 处构造改为 `{ expr, span: attr.span }`；`parse_each_expr` 返回值不变，span 由调用方传入 |
| `crates/engine/src/compiler/codegen/node.rs`                                   | \~20 处 `Directive::If(c)` → `Directive::If { expr, .. }`                                          |
| `crates/engine/src/compiler/codegen/model.rs`                                  | 3 处 `Directive::Model`                                                                            |
| `crates/engine/src/compiler/codegen/once.rs`                                   | 6 处                                                                                               |
| `crates/engine/src/compiler/codegen/shell.rs`                                  | 2 处 + 合成 `Directive::Each { clause, span: Span::empty() }`                                        |
| `crates/engine/src/compiler/validator.rs`                                      | 4 处                                                                                               |
| `crates/engine/src/compiler/component.rs`                                      | 2 处 `Ref` + 合成                                                                                    |
| `crates/engine/src/compiler/code_editor/gen.rs`                                | 1 处                                                                                               |
| `crates/engine/src/compiler/menu/{menu_bar,item}.rs`                           | 2 处                                                                                               |
| `crates/engine/src/compiler/tab_bar/{tab,tab_item}.rs`                         | 4 处                                                                                               |
| `crates/engine/src/compiler/{description_list,table,tab_bar,accordion}/gen.rs` | 合成处补 `span: Span::empty()`                                                                        |
| `crates/engine/src/parser/mod.rs` tests                                        | \~10 处 `matches!` 断言加 `..`                                                                        |
| `crates/lsp/src/semantics/binder.rs`                                           | 匹配臂更新，使用 `span` 字段替代 `elem.span`                                                                  |

### 验证

* `cargo test -p rust-rml-engine` — parser/codegen 全量回归

* 新增测试 `directive_span_covers_attr`：`<div if={x}>` 的 `Directive::If.span` 等于 `attr.span`

***

## Phase 2：tree-sitter-rml 独立 grammar

### 前置

```powershell
npm install -g tree-sitter-cli
```

生成 `parser.c` 后 check-in，后续编译无需 CLI。

### 新 crate 结构

```
crates/tree-sitter-rml/
├── Cargo.toml          # build = "build.rs", deps: tree-sitter (版本对齐 Cargo.lock), cc
├── build.rs            # cc::Build.file("src/parser.c").compile("tree-sitter-rml")
├── grammar.js          # RML 语法定义
├── package.json        # tree-sitter CLI 元数据
├── src/
│   ├── parser.c        # tree-sitter generate 产物（check-in）
│   └── tree_sitter/parser.h
├── lib.rs              # pub const LANGUAGE: LanguageFn; pub const HIGHLIGHTS_QUERY: &str
└── queries/
    ├── highlights.scm  # 41 标准捕获名
    └── injections.scm  # {expr} 注入 rust 语言
```

### grammar.js 核心规则

```javascript
module.exports = grammar({
  name: 'rml',
  extras: $ => [/\s+/],
  rules: {
    document: $ => repeat(choice($.element, $.text, $.comment)),
    element: $ => choice(
      $.self_closing_element,
      seq($.start_tag, repeat(choice($.element, $.text, $.interpolation, $.comment)), $.end_tag),
    ),
    start_tag: $ => seq('<', $.tag_name, repeat($.attribute), '>'),
    self_closing_element: $ => seq('<', $.tag_name, repeat($.attribute), '/>'),
    end_tag: $ => seq('</', $.tag_name, '>'),
    tag_name: $ => /[A-Za-z][A-Za-z0-9_-]*/,
    attribute: $ => seq($.attribute_name, optional(seq('=', $.attribute_value))),
    attribute_name: $ => /[A-Za-z_][A-Za-z0-9_:.-]*/,
    attribute_value: $ => choice($.string, $.binding),
    string: $ => choice(seq('"', /[^"]*/, '"'), seq("'", /[^']*/, "'")),
    binding: $ => seq('{', optional($.expression), '}'),
    interpolation: $ => seq('{', optional($.expression), '}'),
    expression: $ => /[^}]+/,
    text: $ => /[^<{]+/,
    comment: $ => token(seq('<!--', /[^]*/, '-->')),
  },
});
```

### highlights.scm 映射

```
(tag_name) @tag
((tag_name) @type (#match? @type "^[A-Z]"))   ; PascalCase = 组件
(attribute_name) @attribute
((attribute_name) @keyword (#match? @keyword "^(if|else|each|model|show|once|html|ref|key|slot)$"))
((attribute_name) @function (#match? @function "^(on[:_]|onclick)"))
(string) @string
(binding (expression) @variable)
(interpolation (expression) @variable)
(comment) @comment
"<" @punctuation.bracket  ">" @punctuation.bracket
"</" @punctuation.bracket  "/>" @punctuation.bracket
"{" @punctuation.bracket  "}" @punctuation.bracket
```

### injections.scm

```
(binding (expression) @injection.content (#set! injection.language "rust"))
(interpolation (expression) @injection.content (#set! injection.language "rust"))
```

### workspace 集成

* `Cargo.toml` `[workspace] members` 加 `"crates/tree-sitter-rml"`

* `[workspace.dependencies]` 加 `tree-sitter-rml = { path = "crates/tree-sitter-rml" }`

* demo `Cargo.toml` 加 `tree-sitter-rml` 依赖

### demo 启动注册（`demo/src/app.rs` 或等价启动点）

```rust
use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
use tree_sitter_rml::{LANGUAGE, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};

LanguageRegistry::singleton().register("rml", &LanguageConfig {
    name: "rml".into(),
    language: tree_sitter::Language::new(LANGUAGE),
    injection_languages: vec!["rust".into()],
    highlights: HIGHLIGHTS_QUERY.into(),
    injections: INJECTIONS_QUERY.into(),
    locals: "".into(),
});
```

### 验证

* `cargo build -p tree-sitter-rml` 通过

* 单元测试：解析 `<div if={x}>{y}</div>` 得到 `(element (start_tag (attribute_name) (binding (expression))) (interpolation (expression)) (end_tag))`

* demo 中 `InputState::code_editor("rml")` 不再 fallback 到 `text`

***

## Phase 3：LSP 语义 Tokens（动态层）

### 3.1 Token 类型定义（新文件 `crates/lsp/src/semantics/tokens.rs`）

```rust
use lsp_types::{SemanticTokenModifier, SemanticTokenType};

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
    SemanticTokenType::NUMBER,     // 9
];

pub const RML_TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,   // bit 0: ref 目标
    SemanticTokenModifier::DEFINITION,    // bit 1: 已解析绑定
    SemanticTokenModifier::DEPRECATED,    // bit 2: 未解析绑定（划线）
    SemanticTokenModifier::MODIFICATION,  // bit 3: model 双向绑定
];

pub struct SpannedSemanticToken {
    pub span: Span,
    pub token_type: u32,
    pub token_modifiers: u32,
}
```

### 3.2 SemanticModel 扩展（`crates/lsp/src/semantics/model.rs`）

```rust
pub struct SemanticModel {
    pub diagnostics: Vec<SemanticDiagnostic>,
    pub tokens: Vec<SpannedSemanticToken>,  // 新增
}
```

### 3.3 binder.rs 返回结构化结果

`bind()` 签名改为返回 `BindingResult`：

```rust
pub struct BindingResult {
    pub diagnostics: Vec<SemanticDiagnostic>,
    pub tokens: Vec<SpannedSemanticToken>,
}
pub fn bind(root: &Node, meta: Option<&HashMap<String, StructMetadata>>) -> BindingResult
```

`bind_element` 发射规则（每个匹配 `Directive`/`Attribute` 时同步 push token）：

| 构造                       | token\_type       | modifiers                 | span 来源                                      |
| ------------------------ | ----------------- | ------------------------- | -------------------------------------------- |
| 指令名（if/each/model/...）   | KEYWORD           | —                         | `Directive::* span` 内按指令名长度切分                |
| 指令绑定表达式根标识符（已解析）         | VARIABLE          | DEFINITION                | `Directive::* span` 内扫描标识符                   |
| 指令绑定表达式根标识符（未解析）         | PROPERTY          | DEPRECATED                | 同上                                           |
| `model` 绑定字段             | VARIABLE          | DEFINITION + MODIFICATION | `Directive::Model.span` 内                    |
| `each` 迭代变量              | VARIABLE          | DECLARATION               | `Directive::Each.span` 内扫描 `item`            |
| `each` 迭代源（已解析）          | VARIABLE          | DEFINITION                | `Directive::Each.span` 内扫描 `iterable`        |
| 属性名                      | ATTRIBUTE         | —                         | `Attribute::* span` 内按 name 长度切分             |
| `Bind` 表达式根标识符           | VARIABLE/PROPERTY | DEFINITION/DEPRECATED     | `Attribute::Bind.span` 内                     |
| `Event` handler 名（已注册命令） | FUNCTION          | DEFINITION                | `Attribute::Event.span` 内                    |
| `Event` handler 名（未注册）   | FUNCTION          | DEPRECATED                | 同上                                           |
| 插值表达式根标识符                | VARIABLE/PROPERTY | DEFINITION/DEPRECATED     | `Interpolation.span` 内                       |
| 组件标签名（PascalCase）        | TYPE              | —                         | `Element.span` 内按 tag 长度                     |
| HTML 标签名                 | TAG               | —                         | 同上                                           |
| 注释                       | COMMENT           | —                         | tokenizer 阶段 `TokenKind::Text` 前的 `<!-- -->` |

**组件 vs HTML 判定**：用 `rust_rml_engine::tags::BuiltinTag::from_str(tag).is_ok()` 判定 HTML 标签，否则视为组件。

### 3.4 ServerCapabilities 声明（`crates/lsp/src/server/connection.rs::build_capabilities`）

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

### 3.5 dispatch.rs 路由

`handle_request` 增加：

```rust
"textDocument/semanticTokens/full" => handlers::semantic_tokens::handle_full(req.params, state)
        .map(|v| v.and_then(|t| serde_json::to_value(t).ok()))
"textDocument/semanticTokens/range" => handlers::semantic_tokens::handle_range(req.params, state)
        .map(|v| v.and_then(|t| serde_json::to_value(t).ok()))
```

### 3.6 新文件 `crates/lsp/src/handlers/semantic_tokens.rs`

```rust
pub fn handle_full(params: Value, state: &mut ServerState) -> Result<Option<SemanticTokens>>;
pub fn handle_range(params: Value, state: &mut ServerState) -> Result<Option<SemanticTokens>>;
```

实现步骤：

1. 从 params 提取 `uri`
2. `workspace.document(&uri)` 取 `Document` 的 `tree` + `semantic`
3. 遍历 `semantic.tokens`（已按 span.start 排序）
4. 用 `conv::span_to_range` 转 LSP `Range`
5. Delta 编码为 `Vec<SemanticToken>`：`delta_line`/`delta_start`/`length`/`token_type`/`token_modifiers_bitset`
6. 返回 `SemanticTokens { data }`

`handle_range` 同理但过滤 `range` 外的 token。

### 3.7 handlers/mod.rs

增加 `pub mod semantic_tokens;`

### 验证

* `cd crates/lsp && cargo test`：新增 `semantic_tokens_full_returns_delta` 测试，断言 `<div if={count}>` 返回 `keyword`(if) + `variable`(count, DEFINITION)

* `cargo check -p rust-rml-lsp` 通过

***

## Phase 4：Demo 集成

### 4.1 LspClient 增加 semantic\_tokens\_full 方法（`demo/src/lsp/lsp_client.rs`）

```rust
pub fn semantic_tokens_full(&self, uri: &Uri) -> Receiver<Result<Value>> {
    let params = serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
    });
    self.send_request("textDocument/semanticTokens/full", params)
}
```

### 4.2 LspClient.initialize 缓存 legend

当前 `initialize` 只 log 响应。改为解析 `result.capabilities.semanticTokensProvider.legend` 并缓存为 `pub semantic_tokens_legend: Option<SemanticTokensLegend>` 字段。

### 4.3 新文件 `demo/src/lsp/semantic_tokens_provider.rs`

```rust
pub struct RmlSemanticTokensProvider {
    client: Arc<LspClient>,
    uri: Uri,
    legend: SemanticTokensLegend,
}

impl DocumentRangeSemanticTokensProvider for RmlSemanticTokensProvider {
    fn legend(&self) -> SemanticTokensLegend { self.legend.clone() }
    fn semantic_tokens(&self, _text: &Rope, _range: Range<usize>,
        _window: &mut Window, cx: &mut App) -> Task<Result<SemanticTokens>> {
        let rx = self.client.semantic_tokens_full(&self.uri);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()??;
            let result: SemanticTokensResult = serde_json::from_value(resp)?;
            Ok(result.into())
        })
    }
}
```

### 4.4 CodeEditorTab 安装 provider（`demo/src/lsp/code_editor_tab.rml.rs`）

在 `CodeEditorTab::new` 的 `InputState` 构造块中，与现有 providers 并列：

```rust
if let Some(legend) = lsp_client.semantic_tokens_legend() {
    state.lsp.semantic_tokens_provider = Some(Rc::new(
        RmlSemanticTokensProvider::new(lsp_client.clone(), uri.clone(), legend),
    ));
}
```

### 4.5 demo/src/lsp/mod.rs

增加 `pub mod semantic_tokens_provider;` + `pub use semantic_tokens_provider::RmlSemanticTokensProvider;`

### 验证

* 启动 demo，打开 LSP Explorer 中的 `.rml` 文件

* 确认静态层：`<div>` 标签着色、`if` 指令为 keyword、字符串为 string（tree-sitter 即时渲染）

* 确认动态层（100ms debounce 后）：`count` 字段为 variable（DEFINITION），未解析字段为 property（DEPRECATED，划线），`<Button>` 为 type，`onclick={fn}` 为 function

* 编辑代码后 `didChange` 触发 `update_semantic_tokens`，动态层刷新

***

## 横切风险与对策

| 风险                                 | 对策                                                                                                                                                         | <br /> | <br />                |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | :----- | :-------------------- |
| tree-sitter 版本不兼容                  | Phase 2 第一步查 `Cargo.lock` 中 `tree-sitter` 版本，`tree-sitter-rml` 的 `tree-sitter` 依赖对齐同主版本                                                                    | <br /> | <br />                |
| `crates/lsp` 在 workspace exclude 中 | `cargo check` 需 `cd crates/lsp && cargo check`；demo 依赖 `rust-rml-lsp` 通过 path 仍可编译（lsp 默认 feature 无重型依赖）                                                   | <br /> | <br />                |
| legend 一致性                         | `RML_TOKEN_TYPES`/`RML_TOKEN_MODIFIERS` 单一信源在 `semantics/tokens.rs`；server 通过 `build_capabilities` 声明，demo 通过 `initialize` 响应透传                            | <br /> | <br />                |
| grammar.js 注释规则                    | tree-sitter regex 不支持 lazy 量词，注释规则用 `token(seq('<!--', /[^]*/, '-->'))` 或 external scanner；若 parser.c 生成失败，回退为 \`token(prec(-1, seq('\<!--', repeat(/\[^-] | -\[^>] | --\[^>]/), '-->')))\` |
| Directive 子 span 精度                | 指令名/表达式根标识符的子 span 在 LSP emitter 内用源码扫描提取（`source[directive.span]` 内 `find` 指令名 + 标识符），失败时退化为整个 `directive.span`                                           | <br /> | <br />                |

***

## 执行顺序

```
Phase 1 (AST span) ────────> Phase 3 (LSP 语义) ───┐
                                                     ├──> Phase 4 (Demo)
Phase 2 (tree-sitter) ──────────────────────────────┘
```

Phase 1 与 Phase 2 互不依赖可并行；Phase 3 必须 Phase 1（需 `Directive.span`）；Phase 4 等 Phase 2+3。建议串行 1→2→3→4 落地。

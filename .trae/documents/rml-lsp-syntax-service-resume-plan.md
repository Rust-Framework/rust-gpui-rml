# RML LSP 语法服务续推进计划

## Context

延续 [rml-lsp-syntax-service-extension-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-lsp-syntax-service-extension-plan.md)，原计划 P0-P2 已基本落地，但**未完成 mod.rs 子模块声明与路由/capability 收尾**，导致 `crates/lsp` 当前无法编译。本计划用于补齐 P1/P2 收尾、验证编译，并推进 P3-P7。

## 当前实际状态（与原计划偏差）

| Phase | 文件状态 | 编译阻塞 |
|---|---|---|
| P0 Cargo.toml | ✅ 已直接定义 version/edition/license，ra_ap_* 注释，rust-backend feature 注释 | 无 |
| P1 ast_util.rs | ✅ 完整（tag_name_span/find_attribute_at_offset/iter_directive_exprs/directive_expr/event_handler_name + 3 测试） | 无 |
| P1 features/definition.rs | ✅ 完整（find_definition/find_definition_rust/find_command_definition/find_expr_at_offset） | 无 |
| P1 handlers/definition.rs | ✅ 完整 | 无 |
| P1 handlers/mod.rs | ✅ 已声明 `pub mod definition;` | 无 |
| P1 dispatch.rs | ✅ 已路由 `textDocument/definition` | 无 |
| P1 features/mod.rs | ❌ **未声明 `pub mod definition;`** | **阻塞编译** |
| P2 features/document_symbol.rs | ✅ 完整 | 无 |
| P2 handlers/document_symbol.rs | ✅ 完整 | 无 |
| P2 handlers/mod.rs | ❌ **未声明 `pub mod document_symbol;`** | **阻塞编译** |
| P2 features/mod.rs | ❌ **未声明 `pub mod document_symbol;`** | **阻塞编译** |
| P2 dispatch.rs | ❌ 未路由 `textDocument/documentSymbol` | 客户端调用得空响应 |
| P2 connection.rs | ❌ 未声明 `document_symbol_provider` | 客户端不识别能力 |
| P3-P7 | ❌ 全部未实现 | — |

**结论**：当前 `cargo build` 必然失败。Phase A 必须先完成。

## 设计决策（沿用原计划，无变更）

- formatting：智能分行（Prettier 风格）
- rename：含跨语言 .rml.rs（RA 不可用时仅 .rml 改名）
- 客户端 UI：CodeEditorTab 菜单 action
- 标签 span：近似推算（不修 engine）
- RA 后端：ra_ap_* 注释降级，所有 RA 相关代码 `#[cfg(feature = "rust-backend")]` 跳过编译，NoopQuery 提供降级实现

## Phase A：P1/P2 收尾 + 编译验证（必修）

### A1. features/mod.rs 补声明

文件：[crates/lsp/src/features/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/features/mod.rs)

```rust
pub mod ast_util;
pub mod completion;
pub mod definition;          // 新增
pub mod document_symbol;     // 新增
pub mod hover;
pub mod source;
```

### A2. handlers/mod.rs 补声明

文件：[crates/lsp/src/handlers/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/handlers/mod.rs)

```rust
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod document_symbol;     // 新增
pub mod hover;
pub mod initialize;
pub mod sync;
```

### A3. dispatch.rs 增 documentSymbol 路由

文件：[crates/lsp/src/server/dispatch.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/dispatch.rs#L26)

在 `"textDocument/definition"` 臂后追加：

```rust
"textDocument/documentSymbol" => {
    handlers::document_symbol::handle_document_symbol(req.params, state)
        .map(|v| v.and_then(|s| serde_json::to_value(s).ok()))
}
```

### A4. connection.rs 增 document_symbol_provider

文件：[crates/lsp/src/server/connection.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/connection.rs#L145)

```rust
definition_provider: Some(lsp_types::OneOf::Left(true)),
references_provider: Some(lsp_types::OneOf::Left(true)),
document_symbol_provider: Some(lsp_types::OneOf::Left(true)),   // 新增
document_formatting_provider: None,
```

### A5. 编译验证

```sh
cd crates/lsp && cargo build
```

**通过条件**：零错误，可少量 warning。Task #1/#2/#3 标记 completed。

## Phase B：P3 references（含 symbol 分类）

### B1. 新建 features/symbol.rs

文件：`crates/lsp/src/features/symbol.rs`

```rust
pub enum Symbol {
    Tag(String),
    Field(String),
    Command(String),
}

/// 在 .rml AST 中识别光标处的符号
pub fn classify_symbol_at(root: &Node, source: &str, offset: usize) -> Option<Symbol>;
```

分类逻辑（用 `ast_util`）：
1. `find_element_at_offset` → 检查 `tag_name_span` 命中 → `Symbol::Tag`
2. `find_attribute_at_offset`：
   - `Attribute::Bind { expr, .. }` → `parse_binding_path(expr).root` → `Symbol::Field`（root 为 builtin 返回 None）
   - `Attribute::Event { handler, .. }` → `event_handler_name` → `Symbol::Command`
   - `Attribute::Static { .. }` → None
3. 命中文本插值 `Interpolation(expr)` → `parse_binding_path(expr).root` → `Symbol::Field`
4. 遍历 `iter_directive_exprs`，若 offset 在 elem.span 内且表达式 root_ident == 候选 → `Symbol::Field`（近似定位，沿用 definition.rs:103 的简化策略）

### B2. 新建 features/references.rs

文件：`crates/lsp/src/features/references.rs`

```rust
pub fn find_references(
    uri: &Url,
    position: Position,
    include_declaration: bool,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Vec<Location>;
```

实现：
1. `classify_symbol_at` 识别符号
2. 全 AST 递归遍历收集引用点：
   - `Symbol::Tag(name)` → 所有 `Element.tag == name` 的 `tag_name_span`（仅开标签，闭标签无 span 信息跳过）
   - `Symbol::Field(name)` → 所有 `Attribute::Bind.expr` / 指令表达式 / 插值中 `parse_binding_path(expr).root == name` 的属性 span（用 `attr_span`）
   - `Symbol::Command(name)` → 所有 `Attribute::Event.handler` 中 `event_handler_name == name` 的属性 span
3. `include_declaration == true` → 头部插入 `features::definition::find_definition` 的 Location
4. `.rml.rs` → 委托 `rust_query.find_references`（trait 新增方法，NoopQuery 返回空 Vec）

### B3. RustSemanticQuery trait 扩展（也为 P6 准备）

文件：[crates/lsp/src/rust/query.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/query.rs#L103)

新增三个方法：
```rust
fn find_references(
    &self,
    uri: &Url,
    pos: Position,
    include_declaration: bool,
) -> Vec<SymbolLocation>;

fn rename_member(
    &self,
    rml_rs_uri: &Url,
    struct_name: &str,
    member: &str,
    new_name: &str,
) -> Vec<lsp_types::TextEdit>;

fn rename_struct(
    &self,
    old_name: &str,
    new_name: &str,
) -> std::collections::HashMap<Url, Vec<lsp_types::TextEdit>>;
```

文件：[crates/lsp/src/rust/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/mod.rs#L32)

NoopQuery 增加降级实现：三者分别返回 `Vec::new()`、`Vec::new()`、`HashMap::new()`。

`RaAdapter`（`#[cfg(feature = "rust-backend")]`）实现暂留 TODO，因为 ra_ap_* 已注释，feature 不可启用。在 `rust/adapter.rs` 顶部 `#[cfg(feature = "rust-backend")]` 块内补齐三个方法体（占位 `todo!()` 或返回空），保证 feature 启用时能编译。

### B4. 新建 handlers/references.rs

文件：`crates/lsp/src/handlers/references.rs`

```rust
pub fn handle_references(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<Vec<Location>>>;
```

按 `doctype::is_rust_codebehind` 分流：
- `.rml.rs` → `state.rust_query.find_references(uri, pos, include_decl)` 转 `Vec<Location>`
- `.rml` → `features::references::find_references(...)`

### B5. mod.rs/dispatch/capability 收尾

- `features/mod.rs` 增 `pub mod symbol;` + `pub mod references;`
- `handlers/mod.rs` 增 `pub mod references;`
- `dispatch.rs` 增 `"textDocument/references"` 路由
- `references_provider` 已声明，无需改 capability

### B6. 验证

- 单元测试：`features::symbol::tests`（Tag/Field/Command 三分支）+ `features::references::tests`（计数）
- `cargo build` + `cargo test -p rust-rml-lsp`

## Phase C：P4 formatting（智能分行）

### C1. 新建 features/formatting.rs

文件：`crates/lsp/src/features/formatting.rs`

```rust
pub fn format_document(
    source: &str,
    options: &FormattingOptions,
) -> Option<Vec<TextEdit>>;
```

实现策略（沿用原计划规则表）：
- 递归 `format_node(node, depth) -> String`
- 缩进：`options.tab_size` 空格（默认 2），不用 tab
- 单属性 + 无 Element 子节点：单行 `<div class="x">text</div>`
- 多属性（≥2）或含 Element 子节点：头标签 `<tag` 后换行；每属性/指令独占一行缩进 +1；`>` 单独一行回缩进；子节点换行缩进 +1；闭标签 `</tag>` 独立行
- 属性顺序：保持 AST 原序
- 文本节点：保留字面量，仅去除每行首尾多余空白
- 插值 `{expr}`：expr 原样保留
- 空元素：`<tag ...></tag>` 同行
- 文件末尾：单个换行符
- 返回单条 `TextEdit { range: 全文档 [0..source.len()], new_text }`

复用 `rust_rml_engine::parser::parse` 解析为 AST；若解析失败（含 ParseError）返回 None（不动文件）。

### C2. 新建 handlers/formatting.rs

文件：`crates/lsp/src/handlers/formatting.rs`

```rust
pub fn handle_formatting(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<Vec<TextEdit>>>;
```

`.rml.rs` → 返回 None（rustfmt 处理）；`.rml` → 调 features。

### C3. mod.rs/dispatch/capability 收尾

- `features/mod.rs` 增 `pub mod formatting;`
- `handlers/mod.rs` 增 `pub mod formatting;`
- `dispatch.rs` 增 `"textDocument/formatting"` 路由
- `connection.rs::build_capabilities` 改 `document_formatting_provider: Some(OneOf::Left(true))`

### C4. 验证

- 单元测试：6 fixture（单属性/多属性/嵌套/文本/空元素/多行）
- 关键反测：格式化后重新 `parse`，无 ParseError
- `cargo test -p rust-rml-lsp formatting`

## Phase D：P5 signature_help

### D1. 新建 features/signature_help.rs

文件：`crates/lsp/src/features/signature_help.rs`

```rust
pub fn signature_help(
    uri: &Url,
    position: Position,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<SignatureHelp>;
```

定位：光标在 `onclick={cmd, args}` 内 → 取 `cmd` → 在 `metadata.commands` 找包含该 cmd 的 struct → `rust_query.command_signature(rs_uri, struct_name, cmd)` 取 `SymbolInfo.type_str` → 构造 `SignatureInformation { label: type_str, parameters: 从 type_str 解析 }`。

简化策略：先用 `find_element_at_offset` + `find_attribute_at_offset` 找命中的事件属性；若未命中事件属性则返回 None。

### D2. 新建 handlers/signature_help.rs

文件：`crates/lsp/src/handlers/signature_help.rs`

`.rml.rs` → 返回 None；`.rml` → 调 features。

### D3. mod.rs/dispatch/capability 收尾

- `features/mod.rs` 增 `pub mod signature_help;`
- `handlers/mod.rs` 增 `pub mod signature_help;`
- `dispatch.rs` 增 `"textDocument/signatureHelp"` 路由
- `connection.rs::build_capabilities` 增：
  ```rust
  signature_help_provider: Some(SignatureHelpProviderCapability::Options(SignatureHelpOptions {
      trigger_characters: Some(vec![",".to_string(), "(".to_string()]),
      retrigger_characters: None,
      work_done_progress_options: Default::default(),
  })),
  ```

### D4. 验证

- 单元测试：`features::signature_help::tests`（mock rust_query 返回固定 SymbolInfo）
- `cargo test -p rust-rml-lsp signature_help`

## Phase E：P6 rename（含跨语言 .rml.rs）

### E1. 新建 features/rename.rs

文件：`crates/lsp/src/features/rename.rs`

```rust
pub fn rename(
    uri: &Url,
    position: Position,
    new_name: &str,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<WorkspaceEdit>;
```

实现：
1. 合法性校验：`new_name` 匹配 `^[A-Za-z_][A-Za-z0-9_]*$`，否则返回 None
2. `classify_symbol_at` 识别符号
3. 调本 crate `features::references::find_references(uri, position, true, ws, rust_query)` 取 .rml 内引用点 → 转为 `TextEdit { range, new_text: new_name }` 插入 `changes[uri]`
4. 跨语言部分：
   - `Symbol::Field(name)` / `Symbol::Command(name)`：`ws.codebehind_uri(uri)` 取 .rml.rs URI → 在 `metadata_for(uri)` 中找包含该 member 的 struct → `rust_query.rename_member(rs_uri, struct_name, member, new_name)` → 插入 `changes[rs_uri]`
   - `Symbol::Tag(old_name)`：`rust_query.rename_struct(old_name, new_name)` → 多文件分组插入 `changes`
5. `.rml.rs` rename 请求 → 返回 None（暂不支持）

### E2. 新建 handlers/rename.rs

文件：`crates/lsp/src/handlers/rename.rs`

```rust
pub fn handle_rename(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<WorkspaceEdit>>;
```

### E3. mod.rs/dispatch/capability 收尾

- `features/mod.rs` 增 `pub mod rename;`
- `handlers/mod.rs` 增 `pub mod rename;`
- `dispatch.rs` 增 `"textDocument/rename"` 路由
- `connection.rs::build_capabilities` 增 `rename_provider: Some(OneOf::Left(true))`

### E4. RaAdapter 实现（feature gated）

文件：[crates/lsp/src/rust/adapter.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/adapter.rs)

在 `#[cfg(feature = "rust-backend")]` 块内补齐 `find_references`/`rename_member`/`rename_struct` 三个方法体。因 ra_ap_* 当前注释，feature 不可启用，实现可暂用 `todo!("RA backend not available: ra_ap_* dependencies commented out")`。这是 P6 的合规收尾——保证 feature 启用时编译失败暴露明确信号，而非默认沉默降级。

### E5. 验证

- 单元测试：`features::rename::tests`（Field/Command/Tag + 合法性校验）
- `cargo test -p rust-rml-lsp rename`

## Phase F：P7 客户端 UI 集成

### F1. LspClient 扩展

文件：[demo/src/lsp/lsp_client.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_client.rs#L221)

沿用 `definition()` 模式新增 5 个薄包装方法：

```rust
pub fn references(&self, uri: &Uri, position: Position, include_decl: bool) -> Receiver<Result<Value>>;
pub fn document_symbol(&self, uri: &Uri) -> Receiver<Result<Value>>;
pub fn formatting(&self, uri: &Uri) -> Receiver<Result<Value>>;
pub fn signature_help(&self, uri: &Uri, position: Position) -> Receiver<Result<Value>>;
pub fn rename(&self, uri: &Uri, position: Position, new_name: &str) -> Receiver<Result<Value>>;
```

### F2. CodeEditorTab 菜单 action

文件：[demo/src/lsp/code_editor_tab.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/code_editor_tab.rml.rs)

新增 5 个 `#[command]` action：
- `on_format_document` → `client.formatting(uri)` → 解析 `TextEdit[]` → 应用到 InputState（`state.update` + `set_text`）
- `on_rename_symbol` → 弹简易 InputState 取 `new_name` → `client.rename` → 解析 `WorkspaceEdit.changes` → 应用到所有受影响文档（MVP 仅当前文档）
- `on_find_references` → `client.references` → 解析 `Location[]` → 在 explorer panel 或新浮层展示
- `on_show_document_symbols` → `client.document_symbol` → 树形展示
- signature_help 由编辑器自动触发（gpui-component code_editor 模式可能内置），否则仅留通道

action 注册方式：`cx.on_action::<Action>(handler)` 在 `CodeEditorTab::new` 内注册，或通过 RML 声明菜单。

注：apply_lsp_edits 的具体实现取决于 gpui-component InputState API，需先查阅 InputState 是否提供 `set_text`/`replace_range` 类方法。若无，MVP 退化为 `set_text(new_full_text)` 一次性替换。

### F3. 验证

- `cd demo && cargo build`
- 手动验证：编辑器内菜单触发 5 个 action

## Phase G：最终集成验证

```sh
cd crates/lsp && cargo build && cargo test
cd demo && cargo build
```

**通过条件**：
- `crates/lsp` 零编译错误，所有单元测试通过
- `demo` 编译通过
- 6 项 LSP 功能 capability 在 initialize 响应中正确声明

## 实施顺序

| 步骤 | Phase | 验证 |
|---|---|---|
| 1 | A：P1/P2 收尾 | `cargo build` 通过 |
| 2 | B：P3 references | `cargo test references symbol` 通过 |
| 3 | C：P4 formatting | `cargo test formatting` 通过 + 重解析无错 |
| 4 | D：P5 signature_help | `cargo test signature_help` 通过 |
| 5 | E：P6 rename | `cargo test rename` 通过 |
| 6 | F：P7 客户端 UI | `demo` 编译通过 |
| 7 | G：最终集成 | 全量 cargo build + cargo test |

每 Phase 独立 commit。

## 风险与降级（沿用原计划）

| 功能 | RA 不可用（NoopQuery）时 |
|---|---|
| definition（标签/事件/绑定 → .rml.rs） | 跨语言跳转返回 None |
| references（.rml） | 正常工作（纯 AST） |
| references（.rml.rs） | 返回空 Vec |
| document_symbol | 正常工作（纯 AST） |
| formatting | 正常工作（纯 AST） |
| signature_help | command_signature 返回 None → 无签名弹窗 |
| rename（.rml 内） | 正常工作 |
| rename（跨语言 .rml.rs） | rename_member/rename_struct 返回空 → 仅 .rml 改名 |

## 假设

1. `rust_rml_engine::parser::parse(text)` 返回 `Result<Node, ParseError>`（或类似），用于 formatting 的反测
2. `Attribute::Bind`/`Attribute::Event`/`Attribute::Static` 各自携带 `span: Span` 字段（ast_util.rs:53 的 `attr_span` 已印证）
3. `EventHandler` 三态：`Ident`/`MethodName`/`WithArgs`（ast_util.rs:83 已印证）
4. `Node::Interpolation(String)` / `Node::MixedText(Vec<TextSegment>)`（definition.rs:104 已印证）
5. `RustSemanticQuery` trait 已含 `command_signature`，signature_help 可直接复用
6. `Workspace::codebehind_uri`/`Workspace::index` 已存在（workspace/mod.rs 已印证）
7. `lsp_types::OneOf`/`SignatureHelpOptions`/`SignatureHelpProviderCapability` 在 lsp-types 0.95 可用

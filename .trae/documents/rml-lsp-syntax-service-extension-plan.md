# RML LSP 语法服务扩展实施计划

## Context

`crates/lsp` 是 RML 的语言服务器（Roslyn CaaS 理念），复用 `rust-rml-engine` 的 parser/validator/props_registry/tags/scanner 作为单一信源。当前已实现 completion/hover/diagnostics 三项 MVP 功能，但存在两类问题：

1. **声明未实现**：`server/connection.rs:145-146` 声明了 `definition_provider` 与 `references_provider` capability，但 `dispatch.rs` 未路由对应 handler，客户端调用得到空响应（demo 端 `RmlDefinitionProvider` 已就绪但实际无效）。
2. **能力缺失**：`document_symbol`、`formatting`、`signature_help`、`rename` 完全未实现，无法满足"完备语法服务"要求。
3. **编译阻塞**：`crates/lsp` 被 workspace `exclude`（规避 RA 重型 git 依赖），但 `Cargo.toml` 仍用 `*.workspace = true` 继承字段，导致 manifest 解析失败，整个 crate 无法构建。

本计划目标：修复编译 + 补齐 6 项 LSP 功能（definition/references/document_symbol/formatting/signature_help/rename），遵循现有 `features/X.rs + handlers/X.rs + dispatch 路由 + capability 声明` 分层模式，最大化复用 engine API 与现有 `crosslang::coordinator`。

## 设计决策（已与用户对齐）

| 决策点 | 选择 | 理由 |
|---|---|---|
| formatting 换行策略 | 智能分行（Prettier 风格） | 单属性/纯文本元素单行；多属性或≥1 Element 子节点换行+缩进 |
| rename 范围 | 含跨语言 .rml.rs | 新增 `RustSemanticQuery::rename_member/rename_struct`，RA 不可用时降级仅 .rml 改名 |
| 客户端 UI | CodeEditorTab 右键菜单 action | 在 [code_editor_tab.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/code_editor_tab.rml.rs) 加 format/rename/find-references/document-symbols action，直驱 `LspClient` + `InputState::apply_lsp_edits` |
| 标签 span 定位 | 近似推算（不修 engine） | 在 `features::ast_util` 按 `elem.span.start + 1 + tag.len()` 推算标签名区间，避免修改 26 个 compiler 文件 |

## Phase 划分

| Phase | 功能 | 优先级 | 估时 | 依赖 |
|---|---|---|---|---|
| P0 | Cargo.toml 编译修复 | 必修 | 0.5h | 无 |
| P1 | definition | 必修 | 2h | P0 |
| P2 | document_symbol | 中 | 1h | P0 |
| P3 | references | 必修 | 3h | P0 |
| P4 | formatting | 中 | 3h | P0 |
| P5 | signature_help | 低 | 1.5h | P0 |
| P6 | rename（含跨语言） | 中 | 3h | P3 |
| P7 | 客户端 UI 集成 | 中 | 2h | P1-P6 |

每 Phase 独立 commit，可独立验证。

## P0：Cargo.toml 编译修复

**修改文件**：[crates/lsp/Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/Cargo.toml)

将继承字段改为直接定义：
- `version.workspace = true` → `version = "0.1.0"`
- `edition.workspace = true` → `edition = "2021"`
- `license.workspace = true` → `license = "MIT"`
- `rust-rml-engine = { workspace = true }` → `rust-rml-engine = { path = "../engine" }`

**验证**：`cargo build` 在 `crates/lsp/` 目录下通过。

## 公共基础设施（P1 起共用）

### 新增 `features/ast_util.rs`

抽出 [features/hover.rs:41](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/features/hover.rs#L41) 的 `find_element_at_offset` 为 `pub`，并新增：

```rust
/// 推算标签名字节区间：elem.span.start + 1（跳过 <）.. +tag.len()
/// 闭标签 </tag> 不在此推算范围（references 单独处理）
pub fn tag_name_span(elem: &Element) -> Span;

/// 在元素属性中定位 offset 命中的属性
pub fn find_attribute_at_offset<'a>(
    elem: &'a Element,
    offset: usize,
) -> Option<&'a Attribute>;

/// 在元素指令中定位 offset 命中的指令表达式
pub fn find_directive_at_offset(
    elem: &Element,
    offset: usize,
) -> Option<&Directive>;

/// 提取事件处理器名（Ident/MethodName/WithArgs 三态统一）
pub fn event_handler_name(h: &EventHandler) -> &str;
```

**修改** [features/hover.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/features/hover.rs)：删除本地 `find_element_at_offset`，改 `use crate::features::ast_util::find_element_at_offset;`。

**修改** [features/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/features/mod.rs)：`pub mod ast_util;`。

## P1：definition

### 新增 `features/definition.rs`

入口 `find_definition(uri, position, ws, rust_query) -> Option<GotoDefinitionResponse>`：

1. `doctype::is_rust_codebehind(uri)` → 委托 `rust_query.goto_definition` 转 `GotoDefinitionResponse::Array`
2. `.rml` 三类识别（用 `ast_util`）：
   - **标签位置**（`tag_name_span.contains(offset)`）→ `crosslang::coordinator::find_component(tag, rust_query)` 转 `Location`
   - **绑定属性/指令/插值**（`find_attribute_at_offset` + `find_directive_at_offset`）→ 提取 `expr` → `crosslang::coordinator::goto_def_for_binding(uri, expr, ws.index(), rust_query)`
   - **事件属性**（`Attribute::Event`）→ 取 `event_handler_name` → 在 `ws.index().metadata_for(uri)` 中找 `commands` 包含该方法 → `rust_query.resolve_member(rs_uri, struct_name, method)` 取 `location`

### 新增 `handlers/definition.rs`

```rust
pub fn handle_definition(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<GotoDefinitionResponse>>;
```

按 [handlers/completion.rs:20](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/handlers/completion.rs#L20) 的 `is_rust_codebehind` 分流模式。

### 路由 & capability

- [dispatch.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/dispatch.rs) 增 `"textDocument/definition"` 臂
- [handlers/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/handlers/mod.rs) 增 `pub mod definition;`
- capability 已声明（`connection.rs:145`），无需改

## P2：document_symbol

### 新增 `features/document_symbol.rs`

纯 AST 遍历，构建 `DocumentSymbolResponse::Nested`：

```rust
fn build_symbol(elem: &Element, source: &str, line_starts: &[u32]) -> DocumentSymbol {
    let range = conv::span_to_range(elem.span, source, line_starts);
    let kind = if tags::is_root_tag(&elem.tag) {
        SymbolKind::MODULE
    } else {
        SymbolKind::CLASS
    };
    let detail = format!("{} attrs, {} children", elem.attributes.len(), elem.children.len());
    let children = elem.children.iter()
        .filter_map(|c| match c {
            Node::Element(e) => Some(build_symbol(e, source, line_starts)),
            _ => None,
        })
        .collect();
    DocumentSymbol {
        name: elem.tag.clone(),
        detail: Some(detail),
        kind,
        range,
        selection_range: range,
        children: if children.is_empty() { None } else { Some(children) },
        ..Default::default()
    }
}
```

### 新增 `handlers/document_symbol.rs`

`.rml` → 调 features；`.rml.rs` → 返回 `None`。

### capability 增量

[connection.rs::build_capabilities](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/connection.rs) 增：
```rust
document_symbol_provider: Some(OneOf::Left(true)),
```

## P3：references

### 新增 `features/symbol.rs`

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
   - `Attribute::Bind { expr, .. }` → `parse_binding_path(expr).root` → `Symbol::Field`
   - `Attribute::Event { handler, .. }` → `event_handler_name` → `Symbol::Command`
3. `find_directive_at_offset`（If/Show/Key/Html/Model/Each.iterable）→ 取表达式根标识符 → `Symbol::Field`
4. 命中文本插值 `Interpolation(expr)` / `MixedText` 段 → `Symbol::Field`

### 新增 `features/references.rs`

```rust
pub fn find_references(
    uri: &Url,
    position: Position,
    include_declaration: bool,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Vec<Location>;
```

遍历 AST 收集引用点：
- `Symbol::Tag(name)` → 所有 `Element.tag == name` 的 `tag_name_span`（开标签 + 闭标签各一）
- `Symbol::Field(name)` → 所有 `Attribute::Bind.expr` / 指令表达式 / 插值中 `root_ident(expr) == name` 的 `value_span`
- `Symbol::Command(name)` → 所有 `Attribute::Event.handler` 中 `event_handler_name == name` 的 `value_span`

`include_declaration == true` 时，调 `features::definition::find_definition` 取定义点插入头部。

`.rml.rs` 文件 → 委托 `rust_query.find_references`（trait 新增方法，NoopQuery 返回空）。

### 新增 `handlers/references.rs` + 路由 + capability

`references_provider` 已声明，无需改 capability。

## P4：formatting（智能分行）

### 新增 `features/formatting.rs`

```rust
pub fn format_document(
    source: &str,
    options: &FormattingOptions,
) -> Option<Vec<TextEdit>>;
```

**规则表**：

| 元素 | 规则 |
|---|---|
| 缩进 | 每层 `options.tab_size` 空格（默认 2），不用 tab |
| 单属性 + 无 Element 子节点 | 单行：`<div class="x">text</div>` |
| 多属性（≥2）或含 Element 子节点 | 头标签 `<tag` 后换行；每属性/指令独占一行缩进 +1；`>` 单独一行回缩进；子节点换行缩进 +1；闭标签 `</tag>` 独立行 |
| 属性顺序 | 保持 AST 原序 |
| 文本节点 | 保留字面量，仅去除每行首尾多余空白 |
| 插值 `{expr}` | expr 原样保留 |
| 空元素 | `<tag ...></tag>` 同行 |
| 文件末尾 | 单个换行符 |

**策略**：递归 `format_node(node, depth) -> String`，最终 `TextEdit { range: 全文档 [0..source.len()], new_text }`。

### 新增 `handlers/formatting.rs`

`.rml` → 调 features；`.rml.rs` → 返回 `None`（Rust 格式化由 rustfmt 处理）。

### capability 增量

`document_formatting_provider: Some(OneOf::Left(true))`（由 `None` 改）。

## P5：signature_help

### 新增 `features/signature_help.rs`

```rust
pub fn signature_help(
    uri: &Url,
    position: Position,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<SignatureHelp>;
```

定位光标在 `onclick={cmd, args}` 内 → 取 `cmd` → 在 `metadata.commands` 找 → `rust_query.command_signature(rs_uri, struct_name, cmd)` 取 `SymbolInfo.type_str` → 构造 `SignatureInformation { label: type_str, parameters: 从 type_str 解析 }`。

`.rml.rs` → 返回 `None`。

### capability 增量

```rust
signature_help_provider: Some(SignatureHelpOptions {
    trigger_characters: Some(vec![",".to_string(), "(".to_string()]),
    retrigger_characters: None,
    work_done_progress_options: Default::default(),
}),
```

## P6：rename（含跨语言）

### `RustSemanticQuery` trait 扩展

[crates/lsp/src/rust/query.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/query.rs) 新增：

```rust
/// .rml.rs 文件内查找符号引用
fn find_references(
    &self,
    uri: &Url,
    pos: Position,
    include_declaration: bool,
) -> Vec<SymbolLocation>;

/// 重命名 struct 字段或 impl 方法（#[command]/#[computed]）
/// 返回该 .rml.rs 内所有需修改的 TextEdit
fn rename_member(
    &self,
    rml_rs_uri: &Url,
    struct_name: &str,
    member: &str,
    new_name: &str,
) -> Vec<lsp_types::TextEdit>;

/// 全 workspace 重命名 #[component] struct
fn rename_struct(
    &self,
    old_name: &str,
    new_name: &str,
) -> std::collections::HashMap<Url, Vec<lsp_types::TextEdit>>;
```

[NoopQuery](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/mod.rs) 降级实现：三者返回空 `Vec`/空 `HashMap`。

[RaAdapter](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/adapter.rs)（`#[cfg(feature = "rust-backend")]`）实现：
- `find_references`：`analysis.find_all_refs` → 转 `SymbolLocation`
- `rename_member`：复用 `resolve_member` HIR 查询路径定位 `field.source`/`f.source`，取 `name()` 的 `TextRange`，生成单条 `TextEdit`
- `rename_struct`：`symbol_search` 精确匹配 struct → 取 name 子区间 → `TextEdit`

### 新增 `features/rename.rs`

```rust
pub fn rename(
    uri: &Url,
    position: Position,
    new_name: &str,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<WorkspaceEdit>;
```

**合法性校验**：`new_name` 匹配 `^[A-Za-z_][A-Za-z0-9_]*$`，否则返回 `None`。

**WorkspaceEdit 构造**：
1. `classify_symbol_at` 识别符号
2. 调本 crate `features::references::find_references(uri, position, true, ws, rust_query)` 取 .rml 内引用点 → 转为 `TextEdit { range, new_text }` 插入 `changes[uri]`
3. 跨语言部分（`Symbol::Field` / `Symbol::Command`）：
   - `ws.codebehind_uri(uri)` 取 .rml.rs URI
   - 在 `metadata_for(uri)` 中找包含该 member 的 struct
   - `rust_query.rename_member(rs_uri, struct_name, member, new_name)` → 插入 `changes[rs_uri]`
4. `Symbol::Tag`：`rust_query.rename_struct(old_name, new_name)` → 多文件分组插入 `changes`

`.rml.rs` rename 请求 → 返回 `None`（暂不支持，降级为 rust-analyzer 处理）。

### capability 增量

```rust
rename_provider: Some(OneOf::Left(true)),
```

## P7：客户端 UI 集成

### LspClient 扩展

[demo/src/lsp/lsp_client.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_client.rs) 沿用 `definition()` 模式新增薄包装：

```rust
pub fn references(&self, uri: &Uri, position: Position, include_decl: bool) -> Receiver<Result<Value>>;
pub fn document_symbol(&self, uri: &Uri) -> Receiver<Result<Value>>;
pub fn formatting(&self, uri: &Uri) -> Receiver<Result<Value>>;
pub fn signature_help(&self, uri: &Uri, position: Position) -> Receiver<Result<Value>>;
pub fn rename(&self, uri: &Uri, position: Position, new_name: &str) -> Receiver<Result<Value>>;
```

### CodeEditorTab 菜单 action

[demo/src/lsp/code_editor_tab.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/code_editor_tab.rml.rs) 新增 5 个 `#[command]`：

- `on_format_document` → `client.formatting(uri)` → `editor_state.update(cx, |s, cx| s.apply_lsp_edits(&edits, window, cx))`
- `on_rename_symbol` → 弹简易 InputState 取 `new_name` → `client.rename` → `apply_lsp_edits`
- `on_find_references` → `client.references` → 在 explorer panel 或新浮层展示 Location 列表
- `on_show_document_symbols` → `client.document_symbol` → 树形展示
- signature_help 由编辑器自动触发（如 gpui-component 支持），否则仅留通道

action 注册方式：在 [code_editor_tab.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/code_editor_tab.rml.rs) 中通过 RML 声明菜单或 `cx.on_action()` 注册。

## dispatch.rs 路由总览

```rust
"textDocument/definition"      => handlers::definition::handle_definition(params, state)
"textDocument/references"      => handlers::references::handle_references(params, state)
"textDocument/documentSymbol"  => handlers::document_symbol::handle_document_symbol(params, state)
"textDocument/formatting"      => handlers::formatting::handle_formatting(params, state)
"textDocument/signatureHelp"   => handlers::signature_help::handle_signature_help(params, state)
"textDocument/rename"          => handlers::rename::handle_rename(params, state)
```

## build_capabilities 总览

```rust
definition_provider: Some(OneOf::Left(true)),                          // 已有
references_provider: Some(OneOf::Left(true)),                          // 已有
document_symbol_provider: Some(OneOf::Left(true)),                     // 新增
document_formatting_provider: Some(OneOf::Left(true)),                 // 由 None 改
signature_help_provider: Some(SignatureHelpOptions {                   // 新增
    trigger_characters: Some(vec![",".to_string(), "(".to_string()]),
    retrigger_characters: None,
    work_done_progress_options: Default::default(),
}),
rename_provider: Some(OneOf::Left(true)),                              // 新增
```

## 风险与降级

| 功能 | RA 不可用（NoopQuery）时 |
|---|---|
| definition（标签/事件/绑定 → .rml.rs） | 跨语言跳转返回 None，无报错 |
| references（.rml） | **正常工作**（纯 AST） |
| references（.rml.rs） | 返回空 Vec |
| document_symbol | **正常工作**（纯 AST） |
| formatting | **正常工作**（纯 AST） |
| signature_help | command_signature 返回 None → 无签名弹窗 |
| rename（.rml 内） | **正常工作** |
| rename（跨语言 .rml.rs） | `rename_member/rename_struct` 返回空 → 仅 .rml 改名 |

## 验证策略

每 Phase 验证：

| Phase | 单元测试 | 集成验证 |
|---|---|---|
| P0 | - | `cargo build` 在 `crates/lsp/` 通过 |
| P1 | `features::definition::tests`：标签/绑定/事件三类 | 人工 Ctrl+Click 跳转 |
| P2 | `features::document_symbol::tests`：嵌套树 | VSCode Outline 面板 |
| P3 | `features::symbol::classify_symbol_at` 全分支 + `features::references::tests` 计数 | 编辑器 Find All References |
| P4 | `features::formatting::tests`：6 fixture（单属性/多属性/嵌套/文本/空元素/多行） | 格式化后重解析无 ParseError |
| P5 | `features::signature_help::tests` | 输入 `,` 弹签名 |
| P6 | `features::rename::tests`：Field/Command/Tag + 合法性 | F2 重命名跨 .rml/.rml.rs |
| P7 | - | 编辑器内菜单触发 5 个 action |

单元测试位于各 `features/X.rs` 的 `#[cfg(test)] mod tests`，构造 mini AST + Mock `RustSemanticQuery`（参考 [crosslang/coordinator.rs:228](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/crosslang/coordinator.rs#L228) 的 `NoopQuery` 模式）。

最终验证：
```sh
cd crates/lsp && cargo build && cargo test
cd demo && cargo build
```

## 实施顺序

1. P0 → 验证 `cargo build` 通过
2. P1 → 验证 definition 三类跳转
3. P2 → 验证 document_symbol 嵌套
4. P3 → 验证 references 计数 + symbol 分类
5. P4 → 验证 formatting 6 fixture
6. P5 → 验证 signature_help
7. P6 → 验证 rename 三类
8. P7 → 验证客户端 UI 5 action

每 Phase 独立 commit。

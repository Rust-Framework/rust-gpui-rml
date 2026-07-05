# RML LSP Feature Gap 修复计划

## Context

LSP 功能在 103 个测试覆盖下已基本可用,但审计发现 3 个 feature gap。经核实:

- **gap 2(include_declaration=false 不过滤光标位置)**:**不是真 gap**,当前实现符合 LSP 规范——`includeDeclaration` 控制是否包含声明点(find_definition 结果),不是光标位置。光标位置如果是引用,理应被收集。跳过。
- **gap 3(客户端 TextEdit 应用)**:`#[command]` 方法无 `&mut Window` 参数,无法调用 `InputState::set_value`。修复需改 RML codegen 让 `#[command]` 透传 window(见 `.trae/documents/mvvm-completion-remaining.md` 方案 A),是独立的架构改动,**本计划不处理**,作为后续工作。
- **gap 1(Interpolation 引用收集)**:**本计划修复**。`Node::Interpolation(String)` 和 `TextSegment::Interpolation(String)` 没有 span,导致 `find_references` 跳过 Interpolation 中的 Field 引用,用户在编辑器用 Find All References 会丢失 `{field}` 形式的引用。

## 修复方案:给 Interpolation 加 span

### 核心改动:AST 加 span 字段

`crates/engine/src/parser/ast.rs`:
- `Node::Interpolation(String)` → `Node::Interpolation { expr: String, span: Span }`
- `TextSegment::Interpolation(String)` → `TextSegment::Interpolation { expr: String, span: Span }`
- 更新 `Display` 实现(L133):`Node::Interpolation { expr, .. }` => `write!(f, "{{{}}}", expr)`

span 覆盖整个 `{expr}`(含花括号),用于 LSP 引用定位。

### Parser 填充 span

`crates/engine/src/parser/mod.rs`:

1. `parse_text_segments(raw: &str)` → `parse_text_segments(raw: &str, base_offset: usize)`
   - 内部从 `raw.chars().peekable()` 改为 `raw.char_indices().peekable()`
   - 遇到 `{` 时记录 `interp_start = i`(相对偏移),`}` 后记录 `interp_end = j + 1`
   - span = `Span::new(base_offset + interp_start, base_offset + interp_end)`
   - 用 struct 变体构造:`TextSegment::Interpolation { expr: expr.trim().to_string(), span }`

2. `parse_text_node(&self, raw: &str)` → `parse_text_node(&self, raw: &str, base_offset: usize)`
   - 传递 base_offset 给 `parse_text_segments`
   - 单段 Interpolation 时:`Node::Interpolation { expr: e.clone(), span }`(从 segment 取 span)

3. 调用点 L113-118(`parse_children` 的 `TokenKind::Text` 分支):
   - 在 `self.advance()` 前记录 `let base_offset = tok.span.start;`
   - 调用 `self.parse_text_node(&text_owned, base_offset)`

### 机械式解构更新(25+ 处)

所有 `Node::Interpolation(expr)` 和 `TextSegment::Interpolation(expr)` 解构改为 struct 变体:

- `Node::Interpolation(expr)` → `Node::Interpolation { expr, .. }`(只读 expr)
- `TextSegment::Interpolation(expr)` → `TextSegment::Interpolation { expr, .. }`

**engine crate**:
- `compiler/codegen/text.rs:19`、`compiler/codegen/once.rs:147,152`、`compiler/codegen/node.rs:46`、`compiler/codegen/mod.rs:153`(测试夹具)
- `compiler/menu/hoist.rs:93,99`
- `compiler/validator.rs:48`(`Node::Interpolation(_)` → `Node::Interpolation { .. }`)
- `compiler/table/template.rs:244,270,291`(测试夹具,用 `Span::empty()`)

**lsp crate**:
- `features/symbol.rs:86,96`
- `features/definition.rs:105,109`(可用新 span 精确定位)
- `features/formatting.rs:53,63,70,113,122`
- `semantics/binder.rs:33,38`

### 核心修复:references.rs 收集 Interpolation 引用

`crates/lsp/src/features/references.rs` L149-170:

当前跳过 Interpolation 节点(注释"无独立 span,跳过")。修复为:

```rust
Node::Interpolation { expr, span } => {
    if let Symbol::Field(name) = c.symbol {
        if let Some(path) = parse_binding_path(expr) {
            if &path.root == name {
                c.push(span.start, span.end);
            }
        }
    }
}
```

`Node::MixedText(segs)` 中的 `TextSegment::Interpolation { expr, span }` 同理收集。

### 测试更新

**parser/mod.rs 测试**(9 处):`matches!(&segs[i], TextSegment::Interpolation(e) if e == "x")` → `matches!(&segs[i], TextSegment::Interpolation { expr, .. } if expr == "x")`

**新增 LSP 测试**:
- `references.rs` 单元测试:验证 `{count}` 形式的 Interpolation 引用被收集
- `complex_scenarios.rs`:用 `complex.rml` 夹具验证 `{title}` 等插值的引用被收集

## 关键文件

| 文件 | 改动类型 |
|------|---------|
| `crates/engine/src/parser/ast.rs` | AST 结构修改(2 处 variant) |
| `crates/engine/src/parser/mod.rs` | parser span 填充 + 9 处测试更新 |
| `crates/engine/src/compiler/codegen/*.rs` | 解构更新(5 处) |
| `crates/engine/src/compiler/menu/hoist.rs` | 解构更新(2 处) |
| `crates/engine/src/compiler/validator.rs` | 模式匹配更新(1 处) |
| `crates/engine/src/compiler/table/template.rs` | 测试夹具更新(3 处) |
| `crates/lsp/src/features/references.rs` | **核心修复**:收集 Interpolation 引用 |
| `crates/lsp/src/features/{symbol,definition,formatting}.rs` | 解构更新 |
| `crates/lsp/src/semantics/binder.rs` | 解构更新 |
| `crates/lsp/tests/complex_scenarios.rs` | 新增 Interpolation 引用测试 |

## 验证

1. `cargo build -p rust-rml-engine` —— engine crate 编译通过
2. `cargo test -p rust-rml-engine` —— engine 所有测试通过(含 parser 9 处测试更新)
3. `cargo test`(在 `crates/lsp/`)—— LSP 所有测试通过(103 + 新增)
4. 新增测试验证:`find_references` 能收集 `<h1>{title}</h1>` 中 `title` 的引用
5. 回归验证:现有 `references_on_field_finds_all_bindings` 等测试仍通过

## 不处理的项目

- **gap 2**:经核实不是真 gap,跳过
- **gap 3**(客户端 TextEdit 应用):需改 RML codegen 让 `#[command]` 透传 window,是独立架构改动,见 `.trae/documents/mvvm-completion-remaining.md` 方案 A,作为后续工作

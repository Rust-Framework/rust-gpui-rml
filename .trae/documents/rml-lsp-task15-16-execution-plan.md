# RML LSP Task #15 + #16 执行计划

## Context

延续上一轮工作。engine crate 已完成 AST 改造(`Node::Interpolation` / `TextSegment::Interpolation` 改为 struct 变体并填充 span),47 个测试全部通过。

现在 LSP crate 处于编译失败状态(因 AST 结构已变),需要:
- **Task #15**:更新 LSP crate 5 个文件共 13 处 Interpolation 解构,并在 `references.rs` 实现核心修复(收集 Interpolation 引用)
- **Task #16**:新增 Interpolation 引用收集测试 + 运行 LSP 全测试套件验证

本计划是 `rml-lsp-feature-gap-fix-plan.md` 的执行细化,聚焦 LSP crate 的精确改动。

## Current State Analysis

经 Grep 确认,LSP crate 中 13 处 `Interpolation(` 旧解构分布在 5 个文件:

| 文件 | 行号 | 当前代码 | 改动类型 |
|------|------|---------|---------|
| `features/references.rs` | 149 | `Node::Interpolation(expr) => {` | **核心修复**(收集引用) |
| `features/references.rs` | 161 | `TextSegment::Interpolation(expr) = seg` | **核心修复**(收集引用) |
| `features/symbol.rs` | 86 | `Node::Interpolation(expr) => {` | 机械解构 |
| `features/symbol.rs` | 96 | `TextSegment::Interpolation(expr) = seg` | 机械解构 |
| `features/definition.rs` | 105 | `Node::Interpolation(expr) => Some(expr),` | 机械解构 |
| `features/definition.rs` | 109 | `TextSegment::Interpolation(expr) => Some(expr.as_str()),` | 机械解构 |
| `features/formatting.rs` | 53 | `Node::Interpolation(expr) => {` | 机械解构 |
| `features/formatting.rs` | 63 | `TextSegment::Interpolation(_) => true,` | 机械解构 |
| `features/formatting.rs` | 70 | `TextSegment::Interpolation(expr) => {` | 机械解构 |
| `features/formatting.rs` | 113 | `Node::Interpolation(expr) => {` | 机械解构 |
| `features/formatting.rs` | 122 | `TextSegment::Interpolation(expr) => {` | 机械解构 |
| `semantics/binder.rs` | 33 | `Node::Interpolation(expr) => {` | 机械解构 |
| `semantics/binder.rs` | 38 | `TextSegment::Interpolation(expr) = seg` | 机械解构 |

## Proposed Changes

### Task #15-A: 机械解构更新(12 处,4 个文件)

#### 1. `crates/lsp/src/features/symbol.rs`

**L86**:
```rust
// 旧:
Node::Interpolation(expr) => {
// 新:
Node::Interpolation { expr, .. } => {
```

**L96**:
```rust
// 旧:
if let rust_rml_engine::parser::ast::TextSegment::Interpolation(expr) = seg {
// 新:
if let rust_rml_engine::parser::ast::TextSegment::Interpolation { expr, .. } = seg {
```

#### 2. `crates/lsp/src/features/definition.rs`

**L105**:
```rust
// 旧:
Node::Interpolation(expr) => Some(expr),
// 新:
Node::Interpolation { expr, .. } => Some(expr.as_str()),
```
说明:`expr` 是 `&String`,用 `as_str()` 显式转 `&str` 匹配返回类型 `Option<&str>`。

**L109**:
```rust
// 旧:
rust_rml_engine::parser::ast::TextSegment::Interpolation(expr) => Some(expr.as_str()),
// 新:
rust_rml_engine::parser::ast::TextSegment::Interpolation { expr, .. } => Some(expr.as_str()),
```

#### 3. `crates/lsp/src/features/formatting.rs`

**L53**:
```rust
// 旧:
Node::Interpolation(expr) => {
// 新:
Node::Interpolation { expr, .. } => {
```

**L63**:
```rust
// 旧:
TextSegment::Interpolation(_) => true,
// 新:
TextSegment::Interpolation { .. } => true,
```

**L70**:
```rust
// 旧:
TextSegment::Interpolation(expr) => {
// 新:
TextSegment::Interpolation { expr, .. } => {
```

**L113**:
```rust
// 旧:
Node::Interpolation(expr) => {
// 新:
Node::Interpolation { expr, .. } => {
```

**L122**:
```rust
// 旧:
TextSegment::Interpolation(expr) => {
// 新:
TextSegment::Interpolation { expr, .. } => {
```

#### 4. `crates/lsp/src/semantics/binder.rs`

**L33-35**(机械解构,保持 `Span::empty()` 不变以最小化改动):
```rust
// 旧:
Node::Interpolation(expr) => {
    check_binding_expr(expr, elem_span_or_default(node), meta, diags);
}
// 新:
Node::Interpolation { expr, .. } => {
    check_binding_expr(expr, elem_span_or_default(node), meta, diags);
}
```

**L38**:
```rust
// 旧:
if let TextSegment::Interpolation(expr) = seg {
// 新:
if let TextSegment::Interpolation { expr, .. } = seg {
```

### Task #15-B: 核心修复 `references.rs`(2 处)

**L149-158** — `Node::Interpolation` 收集引用:

当前代码:
```rust
Node::Interpolation(expr) => {
    if let Symbol::Field(name) = c.symbol {
        if let Some(path) = parse_binding_path(expr) {
            if &path.root == name {
                // Interpolation 无独立 span，跳过精确位置（无法定位）
                // 这里不收集，避免误报整段文本
            }
        }
    }
}
```

替换为:
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

**L159-171** — `Node::MixedText` 中的 `TextSegment::Interpolation` 收集引用:

当前代码:
```rust
Node::MixedText(segs) => {
    for seg in segs {
        if let rust_rml_engine::parser::ast::TextSegment::Interpolation(expr) = seg {
            if let Symbol::Field(name) = c.symbol {
                if let Some(path) = parse_binding_path(expr) {
                    if &path.root == name {
                        // MixedText 段无独立 span，跳过
                    }
                }
            }
        }
    }
}
```

替换为:
```rust
Node::MixedText(segs) => {
    for seg in segs {
        if let rust_rml_engine::parser::ast::TextSegment::Interpolation { expr, span } = seg {
            if let Symbol::Field(name) = c.symbol {
                if let Some(path) = parse_binding_path(expr) {
                    if &path.root == name {
                        c.push(span.start, span.end);
                    }
                }
            }
        }
    }
}
```

### Task #16-A: 新增 Interpolation 引用收集单元测试

在 `crates/lsp/src/features/references.rs` 的 `tests` 模块末尾追加 2 个测试:

```rust
#[test]
fn find_field_references_in_interpolation() {
    // {count} 形式的插值应被 references 收集
    let rml = Url::parse("file:///x.rml").unwrap();
    let source = "<component><h1>{count}</h1><span>{count}</span></component>";
    let ws = ws_with_doc(&rml, source);
    let q = NoopQuery;

    let doc = ws.document(&rml).unwrap();
    let root = doc.tree.root.as_ref().unwrap();
    // 定位到第一个 {count} 的位置
    let interp_offset = source.find("{count}").unwrap();
    // 光标在 {count} 中间（offset + 2，落在 count 标识符上）
    let cursor_offset = interp_offset + 2;
    let pos = offset_to_position(cursor_offset, source, &doc.tree.line_starts);
    assert_eq!(
        classify_symbol_at(root, source, cursor_offset),
        Some(Symbol::Field("count".to_string()))
    );
    let locs = find_references(&rml, pos, false, &ws, &q);
    assert_eq!(locs.len(), 2, "should find both {{count}} interpolations");
}

#[test]
fn find_field_references_in_mixed_text() {
    // 混合文本 "Count: {count}" 中的 {count} 应被 references 收集
    let rml = Url::parse("file:///x.rml").unwrap();
    let source = "<component><p>Total: {count}</p><p>Sum: {count}</p></component>";
    let ws = ws_with_doc(&rml, source);
    let q = NoopQuery;

    let doc = ws.document(&rml).unwrap();
    let root = doc.tree.root.as_ref().unwrap();
    let interp_offset = source.find("{count}").unwrap();
    let cursor_offset = interp_offset + 2;
    let pos = offset_to_position(cursor_offset, source, &doc.tree.line_starts);
    assert_eq!(
        classify_symbol_at(root, source, cursor_offset),
        Some(Symbol::Field("count".to_string()))
    );
    let locs = find_references(&rml, pos, false, &ws, &q);
    assert_eq!(locs.len(), 2, "should find both mixed-text {{count}} interpolations");
}
```

### Task #16-B: 新增 complex.rml 夹具集成测试

在 `crates/lsp/tests/complex_scenarios.rs` 追加 1 个测试,验证 `complex.rml` 中的 `{title}` 插值引用被收集:

```rust
#[test]
fn references_find_interpolation_in_complex_fixture() {
    // complex.rml 中 {title} 出现在 <h1>{title}</h1> 和 <span>{title}</span>
    // references 应收集到这 2 个插值引用
    let uri = rml_uri();
    let ws = ws_with_doc(&uri, COMPLEX_RML);
    let q = NoopQuery;

    let source = COMPLEX_RML;
    let doc = ws.document(&uri).unwrap();
    let root = doc.tree.root.as_ref().unwrap();

    // 定位到 <h1>{title}</h1> 中的 {title}
    let interp_offset = source.find("{title}").expect("should find {title} in fixture");
    let cursor_offset = interp_offset + 2; // 光标在 title 标识符上
    let pos = offset_to_position(cursor_offset, source, &doc.tree.line_starts);
    assert_eq!(
        classify_symbol_at(root, source, cursor_offset),
        Some(Symbol::Field("title".to_string()))
    );

    let refs = find_references(&uri, pos, false, &ws, &q);
    // complex.rml 中 {title} 出现 2 次：<h1>{title}</h1> 和 <span>{title}</span>
    assert_eq!(refs.len(), 2, "should find 2 {{title}} interpolations in fixture");
}
```

### Task #16-C: 全测试套件验证

1. `cargo build -p rml-lsp` — LSP crate 编译通过
2. `cargo test -p rml-lsp` — 全部测试通过(103 原有 + 3 新增 = 106)
3. `cargo build -p rust-rml-engine` — engine crate 仍编译通过(回归验证)
4. `cargo test -p rust-rml-engine` — engine crate 47 个测试仍通过(回归验证)

## Assumptions & Decisions

1. **span 范围**:`Interpolation` 的 span 覆盖整个 `{expr}`(含花括号),与 bind attr 的 span(覆盖 `name={expr}`)语义一致,都是"引用所在的整体范围"。LSP 客户端会高亮整个范围,符合预期。

2. **binder.rs 不改 span 使用**:虽然现在有真实 span 可用于更精确的诊断,但本计划只做机械解构(保持 `Span::empty()`),以最小化改动范围。改进诊断精度是独立的优化,不在本 gap 修复范围内。

3. **definition.rs 不改逻辑**:虽然现在可以用 span 精确判断光标是否在 Interpolation 范围内,但当前 MVP 简化策略("MixedText 返回第一个插值")仍可接受,不在本 gap 修复范围内。

4. **测试不依赖 NoopQuery 的 find_definition**:`include_declaration=false` 时只走引用收集路径,不受 NoopQuery 影响。

5. **complex.rml 夹具已含 `{title}`**:L3 `<h1>{title}</h1>` 和 L5 `<span>{title}</span>`,共 2 处,适合做集成测试。

## Verification Steps

1. **编译验证**:`cargo build -p rml-lsp` 成功,无 E0164("expected tuple struct or tuple variant")错误
2. **单元测试**:`find_field_references_in_interpolation` 和 `find_field_references_in_mixed_text` 通过
3. **集成测试**:`references_find_interpolation_in_complex_fixture` 通过
4. **回归测试**:原有 `references_on_field_finds_all_bindings`、`find_field_references_in_bind_attrs` 等测试仍通过
5. **全量验证**:`cargo test -p rml-lsp` 全部 106 个测试通过

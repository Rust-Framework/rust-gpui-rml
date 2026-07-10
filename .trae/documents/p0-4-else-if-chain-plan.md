# P0-4: else-if 链式条件渲染指令

## Context

RML 框架当前支持 `if`/`else` 二分支条件渲染，但**不支持 `else if` 链式条件**。文档 `docs/02-syntax/directives.md:37` 明确声明"RML 不支持 else if，需要用嵌套 if 实现"。多分支场景只能用多个并列 `if`（如 `conditional_case.rml` 的三个 `if={tab_index == N}`），语义不清晰且每次都独立判断。

本次迭代新增 `else-if={cond}` 指令，支持 `if`/`else-if`/`else` 链式条件渲染，生成 `if ... else if ... else ...` Rust 表达式。

## 语法设计

```html
<div if={status == "loading"}>加载中...</div>
<div else-if={status == "success"}>成功：{data}</div>
<div else-if={status == "error"}>失败：{error}</div>
<div else>未知状态</div>
```

- 属性名 `else-if`（kebab-case），经 `normalize_attr_name` 规范化为 `else_if`
- 值为 Binding 表达式 `={cond}`
- 与 Vue 的 `v-else-if` 语义一致

## 实施步骤

### 1. AST 新增变体 — `crates/engine/src/parser/ast.rs`

在 `Directive::Else` 后新增：
```rust
/// `else-if={cond}` 链式条件分支
ElseIf { expr: String, span: Span },
```

### 2. Parser 解析 — `crates/engine/src/parser/mod.rs:204-209`

在 `"if"` 分支后、`"else"` 分支**之前**插入 `"else_if"` 分支（顺序关键，否则 `"else"` 抢先命中）：
```rust
"else_if" => {
    if let AttrValue::Binding(expr) = attr.value {
        directives.push(Directive::ElseIf { expr, span: attr.span });
    }
}
```

### 3. Codegen 链式配对 — `crates/engine/src/compiler/translator/builtin/meta.rs:173-268`

替换现有 4a/4b 两段为统一的链扫描算法：

```
i = 0
while i < children.len():
    child = children[i]
    if child 含 If 指令:
        chain = [child]  # 从 if 开始
        j = i + 1
        while j < children.len():
            next = children[j]
            if next 含 ElseIf(无 If): chain.push(next); j += 1
            elif next 含 Else(无 If): chain.push(next); j += 1; break
            else: break
        if chain.len() > 1:  # 有 else-if 或 else
            校验: 链中任一 elem 含 Each → 报错
            parts = []
            for (k, elem) in chain.enumerate():
                clone = elem.clone(); retain 去除 If/ElseIf/Else
                (code, is_iter) = gen_node_impl(clone, ...)
                is_iter → 报错
                if k == 0:         parts.push("if {cond} { {code}.into_any_element() }")
                elif 有 cond:       parts.push(" else if {cond} { {code}.into_any_element() }")
                else (else 分支):   parts.push(" else { {code}.into_any_element() }")
            code.push_str(&format!(".child({})", parts.join("")));
            i = j; continue
    # 独立 else-if/else（未被链消费）→ 报错
    if child 含 ElseIf(无 If): 报错 "else-if 必须紧跟 if 或 else-if"
    if child 含 Else(无 If):   报错 "else 必须紧跟 if 或 else-if"
    # 默认处理（4c）
    ...; i += 1
```

校验靠主循环自然实现：链从 `if` 向前扫描消费 else-if/else；任何被 `i` 主指针命中的 else-if/else 必是孤立项。`else` 后再出现 else-if 时，else 触发 break，后续 else-if 不被消费 → 命中独立检查报错。

单独 `if`（无后续 else-if/else）落入默认处理，仍由 `meta.rs:285` 现有逻辑包 `else { Empty }`。

### 4. Validator — `crates/engine/src/compiler/validator.rs:86`

match 臂追加 `Directive::ElseIf { .. }`：
```rust
Directive::If { .. } | Directive::Each { .. } | Directive::Else { .. } | Directive::ElseIf { .. }
| Directive::Once { .. } | Directive::Html { .. } | Directive::Key { .. } | Directive::Show { .. } => {}
```

### 5. Printer — 两处

**`crates/engine/src/compiler/translator/utils.rs:61`** 和 **`meta.rs:385`**，在 `Directive::Else` 臂后添加：
```rust
Directive::ElseIf { expr, .. } => out.push_str(&format!(" else-if={{{}}}", expr)),
```

### 6. LSP — 3 个文件

**`crates/lsp/src/semantics/binder.rs`**（3 处）：
- 第 128 行 match：仿 `If` 臂添加 `Directive::ElseIf { expr, .. }` → emit "else-if" keyword + check_binding_expr_emit
- 第 207 行诊断 match：在 `If | Show | Key` 臂补 `ElseIf`
- 第 222 行 `directive_span`：or 链加 `Directive::ElseIf { span, .. }`

**`crates/lsp/src/features/formatting.rs:221`**：`format_directive` 在 `Else` 臂后加 `Directive::ElseIf { expr, .. }` → 输出 `else-if={expr}`

**`crates/lsp/src/features/ast_util.rs:192`**：`directive_expr` 在 `If | Show | Key | Html` 臂补 `ElseIf`；测试 `directive_expr_extracts` 补一条 `ElseIf` 断言

### 7. Demo — `demo/src/cases/conditional_case.rml:16-24`

三个并列 `if` 改为 `if`/`else-if`/`else-if` 链：
```html
<Card title="概览内容" if={tab_index == 0}>...</Card>
<Card title="详情内容" else-if={tab_index == 1}>...</Card>
<Card title="设置内容" else-if={tab_index == 2}>...</Card>
```
ViewModel（`.rml.rs`）无需改动。

### 8. 文档 — `docs/02-syntax/directives.md:35-51`

删除"RML 不支持 else if"段落，改写多分支示例为 `if`/`else-if`/`else` 链。指令总览表（第 7-18 行）添加 `else-if` 行。

### 9. 测试

**`crates/engine/src/parser/mod.rs`** tests：新增 `parse_else_if_directive` — 验证 `else-if={cond}` 解析为 `Directive::ElseIf { expr }`

**`crates/engine/src/compiler/codegen/node.rs`** tests：新增：
- `else_if_chain_generates_else_if` — `if` + `else-if` + `else` 生成 `if ... else if ... else ...`
- `multiple_else_if_chain` — 多个 `else-if`
- `standalone_else_if_returns_error` — 独立 `else-if` 报错
- `else_if_after_else_returns_error` — `else` 后跟 `else-if` 报错
- `else_if_chain_with_each_returns_error` — 链含 `each` 报错

## 验证步骤

1. `cargo build` — 确认所有 exhaustive match 编译通过
2. `cargo test -p rust-rml-engine --lib` — parser + codegen 测试全通过
3. `cargo build -p rust-rml-demo` — conditional_case 编译成功
4. 运行 demo — 三 tab 切换互斥渲染正确
5. 造错例 `.rml`（孤立 else-if）确认报错信息正确

## 关键文件清单

| 文件 | 改动 |
|------|------|
| `crates/engine/src/parser/ast.rs` | 新增 `Directive::ElseIf` 变体 |
| `crates/engine/src/parser/mod.rs` | 新增 `else_if` 解析分支 + 测试 |
| `crates/engine/src/compiler/translator/builtin/meta.rs` | codegen 链式配对算法 + printer |
| `crates/engine/src/compiler/validator.rs` | match 臂添加 `ElseIf` |
| `crates/engine/src/compiler/translator/utils.rs` | printer 添加 `ElseIf` |
| `crates/engine/src/compiler/codegen/node.rs` | 新增 else-if 测试 |
| `crates/lsp/src/semantics/binder.rs` | 3 处 match 更新 |
| `crates/lsp/src/features/formatting.rs` | format_directive 添加分支 |
| `crates/lsp/src/features/ast_util.rs` | directive_expr + 测试 |
| `demo/src/cases/conditional_case.rml` | 改用 else-if 链 |
| `docs/02-syntax/directives.md` | 更新文档 |

# 10 Sourcemap 与调试支持

RML 框架的声明式代码生成（`.rml` → `.rml.rs`）必须为调试器（DAP）提供源映射，
使用户在 `.rml` 文件中设置断点时，调试器能在生成的 `.rml.rs` 对应位置暂停。

## 核心约束

**组件开发与迭代必须兼顾 sourcemap 透传**。任何 codegen 改动（新增组件、修改生成逻辑、
调整子节点处理）都必须保证 sourcemap 标记正确透传，否则会破坏调试能力。

## Sourcemap 数据链路

```
.rml 文件
  ↓ parser::parse
AST (Element/Attribute/Directive 节点携带 Span)
  ↓ codegen::gen_node_impl（注入 /*__rml_sm:S:E*/ 标记）
生成的 .rml.rs 代码（临时含标记）
  ↓ codegen::postprocess_sourcemap（扫描标记 → 记录到 SourceMap → 删除标记）
干净的 .rml.rs + SourceMap { entries: Vec<SourceMapEntry> }
  ↓ build.rs::compile（序列化为 JSON）
.rml.rs 文件 + .rml.map 文件
  ↓ dap::LineAccurateMapper（加载 .rml.map）
SourceMapper trait（rml_to_rust / rust_to_rml 双向查询）
```

## 关键实现位置

### 1. AST Span 携带

| 节点类型 | Span 字段 | 定义位置 |
|---------|----------|---------|
| `Element.span` | 覆盖 `<tag ...>...</tag>` | `parser/ast.rs` |
| `Attribute::Static/Bind/Event.span` | 属性名+值区间 | `parser/ast.rs` |
| `Directive::*.span` | 指令名+值区间 | `parser/ast.rs` |
| `Node::Interpolation.span` | `{expr}` 区间 | `parser/ast.rs` |
| `TextSegment::Interpolation.span` | 混合文本插值段 | `parser/ast.rs` |

**约束**：新增 AST 节点类型时，必须携带 `span: Span` 字段，否则该节点无法参与 sourcemap。

### 2. codegen 标记注入

**位置**：`crates/engine/src/compiler/codegen/node.rs::gen_node_impl`

gen_node_impl 是所有节点生成的公共入口。在 Element 和 Interpolation 分支，
生成的 code 字符串前注入行内注释标记：

```rust
Node::Element(elem) => {
    let (code, is_iter) = gen_element(elem, ctx, depth, id_counter, loop_vars, parents)?;
    let marked = format!("/*__rml_sm:{}:{}*/{}", elem.span.start, elem.span.end, code);
    Ok((marked, is_iter))
}
Node::Interpolation { expr, span } => {
    let code = format!("format!(\"{{}}\", {})", gen_expr_code(expr, &lv, &computed));
    let marked = format!("/*__rml_sm:{}:{}*/{}", span.start, span.end, code);
    Ok((marked, false))
}
```

**约束**：
- 新增 Node 变体时，若携带 span，必须在 gen_node_impl 中添加标记注入分支
- 标记格式固定为 `/*__rml_sm:{start}:{end}*/`，后处理扫描依赖此格式
- 标记必须注入到 code 开头（非末尾），确保 rust_line 精确到元素构造起始行

### 3. 后处理扫描

**位置**：`crates/engine/src/compiler/codegen/mod.rs::postprocess_sourcemap`

codegen 主入口在生成完所有代码后调用 postprocess_sourcemap：
1. 用正则 `/\*__rml_sm:(\d+):(\d+)\*/` 扫描所有标记
2. 对每个标记，计算其所在行号与列号（1-based），记录到 `ctx.source_map`
3. 从 out 中删除所有标记（保持行号不变，因为标记是行内的）

**约束**：
- postprocess_sourcemap 必须在 codegen 返回前调用，否则生成的 .rml.rs 包含标记
- 标记删除后行号不变（行内注释，不占整行）

### 4. compile() 接口

**位置**：`crates/engine/src/compiler/mod.rs::compile`

```rust
pub fn compile(source: &str, ctx: &CodegenCtx) -> Result<CompileOutput, CompileError>

pub struct CompileOutput {
    pub code: String,                  // 生成的 Rust 代码（无标记）
    pub source_map: SourceMap,         // .rml span → .rml.rs (line, col) 映射
}
```

### 5. .rml.map 持久化

**位置**：`crates/engine/src/build/mod.rs::Builder::build`

compile 成功后，将 `output.source_map.to_json()` 写入与 `.rml.rs` 同目录的 `.rml.map` 文件。

### 6. dap 消费

**位置**：`crates/dap/src/source_map/mapper.rs`

`SourceMapper` trait 提供 (uri, line, column) 粒度的双向查询：
- `rml_to_rust(uri, line, col) -> Option<(Url, line, col)>`：正向，断点 → 生成代码
- `rust_to_rml(uri, line, col) -> Option<(Url, line, col)>`：反向，栈帧 → 源码

`FilePairMapper`（MVP）：文件级配对，行号原样传递。
`LineAccurateMapper`（待实现）：加载 `.rml.map`，行级精确映射。

## CodegenError 源码位置

**位置**：`crates/engine/src/compiler/mod.rs::CodegenError`

```rust
pub struct CodegenError {
    pub message: String,
    pub span: Option<Span>,  // 错误对应的 .rml 源码区间
}

impl CodegenError {
    pub fn with_span(self, span: Span) -> Self { ... }
}
```

**约束**：所有 codegen 报错路径（如"未知标签"、"缺少必要属性"、"else 无前置 if"）
必须透传 AST 节点的 `elem.span`，便于上层（build.rs / LSP）定位到具体源码位置。

构造方式：
```rust
// ✅ 正确：透传 elem.span
return Err(CodegenError {
    message: format!("unknown tag: <{}>", tag),
    span: Some(elem.span),
});

// ❌ 错误：丢失 span
return Err(CodegenError {
    message: format!("unknown tag: <{}>", tag),
    span: None,  // 仅用于无 AST 节点可关联的逻辑错误
});
```

## 新组件开发 sourcemap 检查清单

新增组件或修改 codegen 逻辑时，按以下清单确认 sourcemap 链路完整：

### 1. AST 节点 span

- [ ] 新增 AST 节点类型时，添加 `span: Span` 字段
- [ ] parser 构造 AST 节点时，正确填充 span（覆盖该节点的字节区间）

### 2. codegen 标记透传

- [ ] 新增 Node 变体时，在 `gen_node_impl` 添加标记注入分支（若携带 span）
- [ ] 新增组件 codegen 函数时，确保通过 `gen_node`/`gen_node_impl` 递归处理子节点
  （不要绕过 gen_node_impl 直接拼接子元素 code，否则子元素无标记）
- [ ] codegen 报错路径透传 `elem.span` 到 `CodegenError.span`

### 3. 后处理完整性

- [ ] codegen 主入口的 `postprocess_sourcemap` 调用未被意外移除
- [ ] 生成的 `.rml.rs` 文件不包含 `__rml_sm:` 字符串（标记已删除）

### 4. 测试验证

- [ ] 新组件的 codegen 测试中，`compile()` 返回的 `source_map.entries` 非空
- [ ] sourcemap 包含新组件对应 AST 节点的 span
- [ ] 直接调用 `gen_component` 等子函数的单元测试，使用 `strip_sourcemap_markers` 清理 code 后再断言

### 5. 调试能力验证

- [ ] 在 `.rml` 文件中为新组件设置断点，调试器能在 `.rml.rs` 对应位置暂停
- [ ] 调试器栈帧能反向映射回 `.rml` 源码位置

## 常见陷阱

### 1. 绕过 gen_node_impl 拼接子元素 code

**陷阱**：组件 codegen 函数直接调用 `gen_element` 处理子节点，绕过 `gen_node_impl`

**后果**：子元素无 sourcemap 标记，调试器无法映射到该子元素

**正确**：通过 `gen_node`（公共入口）处理子节点，让标记注入逻辑统一生效

```rust
// ❌ 错误：绕过 gen_node_impl
for child in &elem.children {
    if let Node::Element(child_elem) = child {
        let (code, _) = gen_element(child_elem, ctx, depth, id_counter, loop_vars, parents)?;
        // code 无 sourcemap 标记
    }
}

// ✅ 正确：通过 gen_node
for child in &elem.children {
    let (code, _) = gen_node(child, ctx, depth, id_counter, loop_vars)?;
    // code 携带 sourcemap 标记
}
```

### 2. 标记格式错误

**陷阱**：手动修改标记格式（如改为 `//__rml_sm:S:E` 行注释）

**后果**：`postprocess_sourcemap` 的正则无法匹配，标记残留或 sourcemap 缺失

**正确**：保持 `/*__rml_sm:{start}:{end}*/` 块注释格式，正则 `/\*__rml_sm:(\d+):(\d+)\*/`

### 3. CodegenError 丢失 span

**陷阱**：codegen 报错时构造 `CodegenError { message, span: None }`

**后果**：build.rs / LSP 无法定位错误到 `.rml` 具体行号

**正确**：透传 `elem.span`（或 `attr.span` / `directive.span`）

### 4. 单元测试断言失败

**陷阱**：直接调用 `gen_component` 的单元测试中，`assert!(code.contains("..."))` 失败

**原因**：code 包含 sourcemap 标记，如 `.child(/*__rml_sm:0:10*/rml_ui::Avatar::new())`

**正确**：在 tests 模块中定义本地 `gen_component` 包装，调用 `strip_sourcemap_markers` 清理后再断言

```rust
#[cfg(test)]
mod tests {
    fn gen_component(...) -> Result<String, CodegenError> {
        let code = super::gen_component(...)?;
        Ok(crate::compiler::codegen::strip_sourcemap_markers(&code))
    }
}
```

## 参考实现

- SourceMap 数据模型：`crates/engine/src/compiler/source_map.rs`
- 标记注入：`crates/engine/src/compiler/codegen/node.rs::gen_node_impl`
- 后处理扫描：`crates/engine/src/compiler/codegen/mod.rs::postprocess_sourcemap`
- compile() 接口：`crates/engine/src/compiler/mod.rs::compile` / `CompileOutput`
- .rml.map 持久化：`crates/engine/src/build/mod.rs::Builder::build`
- dap 消费接口：`crates/dap/src/source_map/mapper.rs::SourceMapper`

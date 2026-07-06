# LSP 语法服务改进方案

> 解决用户反馈的 5 个问题，从「开发者语法服务」视角重构 LSP demo 体验。

## 一、问题清单

| # | 问题 | 影响范围 |
|---|------|---------|
| 1 | 打开 RML 文件后 CodeEditor 未占满宽高 | 布局 |
| 2 | 鼠标移入 tag / property / property-value 时 QuickInfo 不切换 | 悬停 |
| 3 | 未单独识别属性名、属性值、绑定表达式等细粒度范围 | 悬停 |
| 4 | 悬停内容不规范（纯文本、字段罗列） | 悬停 |
| 5 | `.rs` 文件无语法服务支持 | Rust 集成 |

## 二、根因分析

### 问题 1：CodeEditor 布局塌陷
- 文件：[demo/assets/styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css#L184-L202)
- 根因：`.lsp-editor-pane` 用 `flex: 1`，但其父容器是 `overflow_y_scroll` 的 case-pane，并非 flex column 容器，故 `flex: 1` 失效，高度退化为内容高度。
- 对照：`.case-pane` 用 `height: 100%` 工作正常。

### 问题 2 & 3：悬停范围错乱
- 文件：[crates/lsp/src/features/hover.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/features/hover.rs#L28-L37)
- 根因：`hover()` 仅用 `elem.span`（整个元素）作 range，且只调用 `format_tag_hover(&elem.tag)`，无论光标落在标签名、属性名还是属性值上，都返回同一份标签文档。
- AST 层面（[crates/engine/src/parser/ast.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/ast.rs#L54-L77)）：`Attribute::Static/Bind/Event` 的 `span` 覆盖「名+值」整体，未单独保存 name/value 子 span，需基于 source 切分。

### 问题 4：内容不规范
- 当前用 `MarkedString::String` 纯文本，列字段以 `- \`name\`` 罗列。
- 应改为 `HoverContents::Markup(MarkupContent)` Markdown，含标题、类型签名、分类章节。

### 问题 5：.rs 无语法服务
- 文件：[crates/lsp/Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/Cargo.toml#L25-L50) 与 [Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/Cargo.toml#L5)
- 根因：`crates/lsp` 被 workspace exclude，`ra_ap_*` 依赖与 `rust-backend` feature 全部注释，运行期 `ServerState` 退化为 `NoopQuery`（[crates/lsp/src/rust/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/rust/mod.rs#L30-L107)），`.rml.rs` 悬停恒返回 `None`。

## 三、设计方案

### Part 1：CodeEditor 布局修复

**改动**：`demo/assets/styles.css` 单点改动。

```css
.lsp-editor-pane {
    height: 100%;      /* 原 flex: 1 */
    min-height: 0;
    width: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
}
.lsp-editor-area {
    flex: 1;            /* 在 lsp-editor-pane 内部撑满 */
    min-height: 0;
    overflow: hidden;
}
```

**验证**：demo 中打开任意 `.rml` 文件，CodeEditor 填满右侧整个编辑区，无空白带、无溢出滚动条。

### Part 2：细粒度悬停（核心）

#### 2.1 AST 工具扩展：`crates/lsp/src/features/ast_util.rs`

新增三个 `pub` 函数（基于 source 文本切分 attribute 整体 span）：

```rust
/// 取属性名 span：在 attr.span.start 处向前扫描源码直到 '=' 或属性 span 结束
pub fn attr_name_span(attr: &Attribute, source: &str) -> Option<Span>;

/// 取属性值 span：定位 '=' 后的内容（剥离引号/大括号）
pub fn attr_value_span(attr: &Attribute, source: &str) -> Option<Span>;

/// 取绑定表达式内部内容 span：`{expr}` 中 expr 的字节区间
pub fn attr_bind_expr_span(attr: &Attribute, source: &str) -> Option<Span>;
```

并公开 `attr_span`（原 `fn` → `pub fn`），供 hover 三级检测复用。

**实现策略**：
- `attr_name_span`：从 `attr.span.start` 开始扫描，遇到 `=`、空格、`/`、`>` 即停，结果 trim 起始空格。
- `attr_value_span`：从 name 结束处找 `=`，跳过 `=` 与可选空格，再根据首字符判断分隔符（`"` 字符串 / `{` 绑定 / `'` 字符串），返回内容区间（不含定界符）。
- `attr_bind_expr_span`：复用 `attr_value_span`，仅当 `Attribute::Bind` 时返回 Some。

#### 2.2 悬停重写：`crates/lsp/src/features/hover.rs`

**三级检测流程**（光标优先级从细到粗）：

```
1. find_attribute_at_offset(elem, offset)
   → 若命中属性：
      a) attr_name_span.contains(offset) → format_attribute_name_hover
      b) attr_value_span.contains(offset) → format_attribute_value_hover
      c) 兜底（落在属性整体 span 但不在 name/value 上） → format_attribute_hover
2. tag_name_span(elem).contains(offset)
   → format_tag_hover
3. 兜底：返回 None（不在任何可悬停范围）
```

**Hover 内容模板**（Markdown，`MarkupContent`）：

- **标签名悬停**：
  ```
  # <Tag>
  Root element | HTML element | Component
  
  ## Attributes
  - `class` (static) — CSS class
  - `value` (bind) — `{field}` 双向绑定
  - `onclick` (event) — 事件处理器
  ```

- **属性名悬停**：
  ```
  ### `attr_name` (static | bind | event)
  
  适用标签: `<Tag>`
  类型: string | bind expression | event handler
  ```

- **属性值悬停**：
  ```
  ### Value of `attr_name`
  
  - 类型: static string | bind expression `{field}` | event handler `fn`
  - 内容: "literal" | field | handler name
  ```

#### 2.3 测试用例（ast_util.rs `#[cfg(test)]`）

```rust
#[test] fn attr_name_span_static()      // class="card" → "class"
#[test] fn attr_value_span_static()     // class="card" → "card"
#[test] fn attr_name_span_bind()        // value={field} → "value"
#[test] fn attr_bind_expr_span_bind()   // value={field} → "field"
#[test] fn attr_name_span_event()       // onclick={fn} → "onclick"
```

### Part 3：handlers/hover.rs 规范化

**问题**：`.rml.rs` 分支仍用 `MarkedString::String` 纯文本，丢失 Markdown 渲染。

**改动**：[crates/lsp/src/handlers/hover.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/handlers/hover.rs#L18-L25)

```rust
if doctype::is_rust_codebehind(&uri) {
    Ok(state.rust_query.hover(&uri, position).map(|info| Hover {
        range: info.range,
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: info.content,
        }),
    }))
}
```

> RA 返回的 `info.content` 本就是 Markdown（adapter.rs `hover()` 已配置 `HoverDocFormat::Markdown`）。

### Part 4：启用 rust-analyzer 集成

#### 4.1 workspace 根 Cargo.toml

```toml
# 原: exclude = ["crates/lsp", "crates/dap"]
exclude = ["crates/dap"]
```

#### 4.2 crates/lsp/Cargo.toml

取消注释 7 行 `ra_ap_*` 依赖（line 31-37）和 `rust-backend` feature 段（line 42-50）。

> **前置条件**：执行 `cargo fetch` 拉取 rust-analyzer git 仓库（约 200MB）。网络受限环境需先解决访问 GitHub 的问题。

#### 4.3 ServerState 启用 RaAdapter

文件 `crates/lsp/src/rust/mod.rs` 已通过 `#[cfg(feature = "rust-backend")]` 自动切换，无需改动。

**验证**：
- `cargo build -p rust-rml-lsp --features rust-backend` 成功
- demo 中打开 `.rml.rs` 文件，hover struct 字段有 RA 返回的 Markdown 文档
- `goto_definition` 在 `.rml.rs` 内可跳转到 struct/field 定义

## 四、实施步骤

| 步骤 | 改动文件 | 验证 |
|------|---------|------|
| 1. CSS 布局修复 | demo/assets/styles.css | 编辑器填满区域 |
| 2. ast_util.rs 新增 3 个 pub fn + 5 个测试 | crates/lsp/src/features/ast_util.rs | `cargo test -p rust-rml-lsp` 通过 |
| 3. hover.rs 三级检测 + Markdown | crates/lsp/src/features/hover.rs | hover tag/attr-name/attr-value 切换 |
| 4. handlers/hover.rs .rml.rs 分支 Markup | crates/lsp/src/handlers/hover.rs | .rml.rs hover 显示 Markdown |
| 5. workspace 解除 exclude | Cargo.toml | `cargo check` 成功 |
| 6. 启用 ra_ap_* 依赖与 feature | crates/lsp/Cargo.toml | `cargo build --features rust-backend` 成功 |
| 7. 端到端验证 | demo | 5 个问题全部解决 |

## 五、风险与缓解

| 风险 | 缓解 |
|------|------|
| rust-analyzer git 依赖拉取失败 | 保留 `default = []` feature，无网络时仍可降级编译 |
| `attr_value_span` 源码切分边界 case 多（空格、引号转义、多行属性） | MVP 阶段先支持单行属性，多行属性 fallback 返回 None |
| `NoopQuery` 与 `RaAdapter` 行为差异大 | feature flag 隔离，编译时确定，无运行期切换风险 |

## 六、不在本次范围

- 跨语言（rml → rml.rs）的 hover 跳转（属 goto_definition 范畴）
- 属性值的语义补全（如 enum 取值提示）
- 指令（if/each/model 等）的 hover（下一阶段）

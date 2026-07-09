# Builtin Translator 修复（Phase 2）

## Summary

承接前序会话，Phase 1（compiler 目录重组）已完成并通过 `cargo check -p rust-rml-engine`。本计划聚焦 Phase 2：修复 `crates/engine/src/compiler/translator/builtin/` 下 6 项元数据/逻辑缺陷（P0×3 + P1×2 + P2×1），按优先级依次执行。

---

## Current State Analysis（已验证）

### 已验证事实

1. **Phase 1 编译通过**：`cargo check -p rust-rml-engine` 仅有 warnings，无 error
2. **`is_self_closing` 是死字段**：Grep `\.is_self_closing`（带点的读取）返回 **0 匹配**；23 处出现全是字段定义（meta.rs:26）或赋值（22 个 translator 文件）
3. **`is_container` 非死字段**：在 meta.rs:35 被 `to_metadata()` 读取 → `.container(self.is_container)`，保留不删
4. **`is_void_tag()` 是 void 标签判定单一信源**：位于 meta.rs:432，硬编码 `input | img | br`
5. **builtin/ 无单元测试**：Grep `#[test]`/`#[cfg(test)]` 返回 0 匹配，删除字段不会破坏测试
6. **`gen_expr_code` 导入路径正确**：deferred.rs:12 和 meta.rs:10 均用 `use crate::compiler::codegen::text::gen_expr_code;`
7. **22 个 translator 文件的 `is_self_closing` 赋值模式完全一致**：每个文件恰好 1 行，位于 `is_container` 与 `is_styled` 之间，4 空格缩进

### 6 项缺陷现状

| ID | 优先级 | 文件 | 现状 |
|----|--------|------|------|
| P0-1 | P0 | `br.rs:12` | `ctor: "gpui::div().hidden()"` → display:none，不渲染不换行 |
| P0-2 | P0 | `img.rs:39-47` | 仅提取 `Attribute::Static` 的 src，`<img src={dyn} />` 被丢弃 |
| P0-3 | P0 | `a.rs` | href 属性被 `apply_static_attr` 静默丢弃，无任何提示 |
| P1-4 | P1 | `meta.rs:25-26` + 22 文件 | `is_self_closing` 死字段，零读取 |
| P1-5 | P1 | `p.rs:12` | `ctor` 强制 `.text_sm().text_color(--text-muted)`，与段落语义冲突 |
| P2-6 | P2 | `deferred.rs:121` | `.container(true)` 缺就近注释，语义易误解 |

---

## Proposed Changes

### Step 1: P0-1 修复 `<br>` 构造器

**文件**：`crates/engine/src/compiler/translator/builtin/br.rs`

**修改**：第 12 行
```rust
// before
ctor: "gpui::div().hidden()",
// after
ctor: "gpui::div().w_full().h_0()",
```

**原理**：GPUI 无原生换行元素。`w_full()` 占满父容器宽度迫使后续内容换到下一行，`h_0()` 不占垂直空间。这是 GPUI 下最接近 `<br>` 视觉效果的实现。`hidden()` 会使元素 `display:none`，既不渲染也不占空间，完全无效。

---

### Step 2: P0-2 修复 `<img>` 支持 bind src

**文件**：`crates/engine/src/compiler/translator/builtin/img.rs`

**修改 2a**：第 10 行 import，增加 `Attribute` 已存在，但需新增 `gen_expr_code` 导入

当前第 7-10 行：
```rust
use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};
```

新增第 11 行（在 `use crate::parser...` 之后）：
```rust
use crate::compiler::codegen::text::gen_expr_code;
```

**修改 2b**：替换 `to_rust` 方法体（第 30-51 行）

当前：
```rust
fn to_rust(
    &self,
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<(String, bool), CodegenError> {
    // 提取 src 属性作为 img() 构造参数
    let src = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "src" => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let ctor = format!("gpui::img({:?})", src);
    super::meta::builtin_engine::translate(
        elem, ctx, id_counter, loop_vars, parents, &ctor, true,
    )
}
```

替换为：
```rust
fn to_rust(
    &self,
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<(String, bool), CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let ctor = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "src" => {
                Some(format!("gpui::img({:?})", value))
            }
            Attribute::Bind { name, expr, .. } if name == "src" => {
                Some(format!("gpui::img({})", gen_expr_code(expr, &lv, &computed)))
            }
            _ => None,
        })
        .unwrap_or_else(|| "gpui::img(\"\")".to_string());

    super::meta::builtin_engine::translate(
        elem, ctx, id_counter, loop_vars, parents, &ctor, true,
    )
}
```

**原理**：`<img src={dynamic} />` 的 `Attribute::Bind` 分支被 `_ => None` 吞掉，导致 ctor 回退为默认 `gpui::img("")`。修复后 bind 表达式经 `gen_expr_code` 生成 Rust 代码作为构造参数。

---

### Step 3: P0-3 修复 `<a>` href 丢弃警告

**文件**：`crates/engine/src/compiler/translator/builtin/a.rs`

**修改 3a**：第 6 行 import，增加 `Attribute`

当前：
```rust
use crate::parser::ast::Element;
```
改为：
```rust
use crate::parser::ast::{Attribute, Element};
```

**修改 3b**：替换 `to_rust` 方法体（第 26-35 行）

当前：
```rust
fn to_rust(
    &self,
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<(String, bool), CodegenError> {
    BuiltinTranslator { meta: META }.to_rust(elem, ctx, id_counter, loop_vars, parents)
}
```

替换为：
```rust
fn to_rust(
    &self,
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<(String, bool), CodegenError> {
    if elem.attributes.iter().any(|a| {
        matches!(a, Attribute::Static { name, .. } | Attribute::Bind { name, .. } if name == "href")
    }) {
        eprintln!(
            "[rml warning] `<a href=\"...\">` is not functional in GPUI; \
             use `<Link href=\"...\">` component for hyperlink behavior. \
             href will be dropped."
        );
    }
    BuiltinTranslator { meta: META }.to_rust(elem, ctx, id_counter, loop_vars, parents)
}
```

**原理**：GPUI `div()` 无超链接能力，href 被静默丢弃。至少应告知开发者属性无效并引导使用 `<Link>` 组件，避免沉默失败。

---

### Step 4: P1-5 修复 `<p>` 强制弱化样式

**文件**：`crates/engine/src/compiler/translator/builtin/p.rs`

**修改**：第 12 行
```rust
// before
ctor: "gpui::div().text_sm().text_color(rml_core::theme::color(\"--text-muted\"))",
// after
ctor: "gpui::div()",
```

**原理**：`<p>` 强制 `text_sm()` + `--text-muted` 与 HTML 语义冲突——段落应有正常字号和正常颜色，样式应由 CSS 控制。退化为普通 `div()` 后，`<p>` 的样式行为与 `<div>` 一致，由开发者通过 class/style 控制。

---

### Step 5: P2-6 `<deferred>` container 语义注释

**文件**：`crates/engine/src/compiler/translator/builtin/deferred.rs`

**修改**：第 120-122 行 `metadata()` 方法

当前：
```rust
fn metadata(&self) -> TranslatorMetadata {
    TranslatorMetadata::new(TAG, DISPLAY_NAME, ComponentCategory::Layout).container(true)
}
```

替换为：
```rust
fn metadata(&self) -> TranslatorMetadata {
    // container=true：含子元素，但子元素作为 gpui::deferred(child) 构造参数传入，
    // 非 ParentElement 的 .child() 链式调用
    TranslatorMetadata::new(TAG, DISPLAY_NAME, ComponentCategory::Layout).container(true)
}
```

**原理**：文件头注释已说明 Deferred 非 ParentElement，但 `metadata()` 的 `.container(true)` 与该语义表面矛盾（container=true 通常暗示 ParentElement）。就近注释消除歧义。

---

### Step 6: P1-4 删除 `is_self_closing` 死字段

**6a. meta.rs —— 删除 struct 字段定义 + 文档注释**

文件：`crates/engine/src/compiler/translator/builtin/meta.rs`

删除第 25-26 行：
```rust
    /// 是否为 void / 自闭合标签
    pub is_self_closing: bool,
```

替换为空（即删除这两行，让 `is_container` 和 `is_styled` 相邻）。

**6b. 22 个 translator 文件 —— 删除 `is_self_closing` 赋值行**

每个文件恰好 1 行，内容为以下之一：
- `    is_self_closing: true,`（20 个文件）
- `    is_self_closing: false,`（2 个文件：anchored.rs, textarea.rs）

**完整文件清单**（22 个）：

| # | 文件 | 值 | 行号 |
|---|------|-----|------|
| 1 | `anchored.rs` | false | 18 |
| 2 | `textarea.rs` | false | 17 |
| 3 | `br.rs` | true | 14 |
| 4 | `a.rs` | true | 14 |
| 5 | `button.rs` | true | 14 |
| 6 | `code.rs` | true | 14 |
| 7 | `div.rs` | true | 14 |
| 8 | `h1.rs` | true | 14 |
| 9 | `h2.rs` | true | 14 |
| 10 | `h3.rs` | true | 14 |
| 11 | `h4.rs` | true | 14 |
| 12 | `h5.rs` | true | 14 |
| 13 | `h6.rs` | true | 14 |
| 14 | `img.rs` | true | 18 |
| 15 | `input.rs` | true | 17 |
| 16 | `label.rs` | true | 14 |
| 17 | `li.rs` | true | 14 |
| 18 | `ol.rs` | true | 14 |
| 19 | `p.rs` | true | 14 |
| 20 | `span.rs` | true | 14 |
| 21 | `svg.rs` | true | 18 |
| 22 | `ul.rs` | true | 14 |

**执行策略**：
- 对每个文件用 Edit 删除 `    is_self_closing: true,\n` 或 `    is_self_closing: false,\n`（含换行符，整行删除）
- 不同文件可并行编辑；同一文件仅 1 处，无冲突风险
- meta.rs 单独处理（删除 2 行：文档注释 + 字段定义）

**原理**：`is_self_closing` 零读取，是纯死字段。void 标签判定由 `is_void_tag()` 函数（meta.rs:432）作为单一信源。删除死字段减少代码噪音，避免误导后续维护者认为该字段有实际作用。

---

### Step 7: 全量验证

```bash
# 1. Phase 2 编译验证
cargo check -p rust-rml-engine

# 2. 组件属性注册护栏（确保重组未破坏注册一致性）
cargo test -p rust-rml-engine --lib props_registry::tests

# 3. 全量工作区编译 + 测试
cargo build --workspace
cargo test --workspace
```

---

## Assumptions & Decisions

1. **`is_container` 保留**：meta.rs:35 `to_metadata()` 读取 `self.is_container`，非死字段，不删除
2. **`is_void_tag()` 保留**：作为 void 标签判定单一信源（meta.rs:432），与 `is_self_closing` 删除无冲突
3. **`<br>` 用 `w_full().h_0()`**：GPUI 无原生换行元素，此为最接近的视觉换行近似
4. **`<a>` href 仅警告不报错**：避免破坏现有使用 `<a>` 的 RML 文件，仅提示开发者
5. **`<p>` 退化为 `div()`**：样式交由 CSS 控制，符合关注点分离
6. **P0 → P1 → P2 执行顺序**：先修逻辑缺陷（P0），再清理死字段（P1），最后补注释（P2）

## Verification Steps

1. `cargo check -p rust-rml-engine` —— 编译通过，无 error
2. `cargo test -p rust-rml-engine --lib props_registry::tests` —— 组件属性注册护栏通过
3. `cargo build --workspace` + `cargo test --workspace` —— 全量无回归

## 风险

- **低风险**：所有改动集中在 `translator/builtin/` 目录，影响面小
- **`is_self_closing` 删除**：已验证零读取，删除后编译器会立即报错若有遗漏的读取点
- **`<br>` 行为变化**：从 `hidden()`（不渲染）变为 `w_full().h_0()`（占宽不占高），是行为修复而非回归

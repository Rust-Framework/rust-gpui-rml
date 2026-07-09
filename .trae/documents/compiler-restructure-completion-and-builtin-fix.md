# Compiler 重组收尾 + Builtin 修复

## Summary

承接前序会话（上下文丢失），完成 `crates/engine/src/compiler/` 目录重组的**剩余路径修复**，随后修复 builtin translator 的 7 个元数据/逻辑缺陷。

当前状态：Phase 1 重组已完成 ~80%（文件移动 ✓、mod.rs ✓、setters.rs 改名 ✓、A/B/C 类路径 ✓）。剩余 17 个文件的路径引用未修复 + Phase 2 builtin 修复未启动。

---

## Current State Analysis（已验证）

### Phase 1 剩余工作：17 个文件路径修复

通过 Grep `super::(super::)?component::` 在 `components/` 目录下发现 18 个文件命中，其中 **`translator/registry.rs:41` 是误报**——该行 `super::component::register_all` 指向 `translator::component::register_all`（已验证 `translator/component/mod.rs:96` 存在 `pub fn register_all`），与重命名的 `compiler::component` → `compiler::setters` 无关，**不得修改**。

**D 类（8 个单文件，`super::component::` → `crate::compiler::setters::`）：**
- `components/alert.rs`（9 处）
- `components/icon.rs`（5 处）
- `components/popover.rs`（4 处）
- `components/tag.rs`（3 处）
- `components/kbd.rs`（4 处）
- `components/label.rs`（4 处）
- `components/separator.rs`（3 处）
- `components/radio_group.rs`（3 处）

**E 类（9 个目录内文件，`super::super::component::` → `crate::compiler::setters::`）：**
- `components/tabs/gen.rs`（3 处）
- `components/tabs/tab.rs`（3 处）
- `components/tabs/setters.rs`（7 处）
- `components/tab_bar/gen.rs`（3 处）
- `components/tab_bar/setters.rs`（3 处）
- `components/table/gen.rs`（3 处）
- `components/table/column.rs`（3 处）
- `components/table/setters.rs`（1 处）
- `components/tree/gen.rs`（3 处）

**关键顺序：** E 类（`super::super::component::`，长串）必须先于 D 类（`super::component::`，短串）替换，因为短串是长串的子串。但由于这两类在不同文件中，实际可并行处理。同一文件内不会同时出现两种模式。

### Phase 2：Builtin 修复（7 项）

**P0-1: `<br>` 无效实现**
- 文件：`translator/builtin/br.rs:12`
- 现状：`ctor: "gpui::div().hidden()"` → `display:none`，不渲染不占空间
- 修复：`ctor: "gpui::div().w_full().h_0()"` —— w_full 占满宽度（视觉换行），h_0 不占高度

**P0-2: `<img>` 不支持 bind src**
- 文件：`translator/builtin/img.rs:38-51`
- 现状：仅提取 `Attribute::Static` 的 src，`<img src={dynamic} />` 被丢弃
- 修复：在 `to_rust` 中增加 `Attribute::Bind` 分支，用 `gen_expr_code` 生成表达式作为构造参数

**P0-3: `<a>` 静默丢弃 href**
- 文件：`translator/builtin/a.rs`
- 现状：href 被 `apply_static_attr` 静默丢弃
- 修复：在 `to_rust` 中检测 href 属性，输出 warning 提示用 `<Link>` 组件

**P1-4: 删除 is_self_closing 死字段**
- 文件：`translator/builtin/meta.rs:26`（字段定义）+ 22 个 translator 文件（字段赋值）
- 验证：Grep `.is_self_closing` 零读取，确认死字段
- 22 个赋值文件：anchored, br, a, code, button, div, h1-h6, input, img, label, li, p, ol, span, svg, textarea, ul
- 修复：删除 struct 字段 + 所有赋值行；保留 `is_void_tag()` 作为 void 标签判定单一信源

**P1-5: `<p>` 强制弱化样式**
- 文件：`translator/builtin/p.rs:12`
- 现状：`ctor: "gpui::div().text_sm().text_color(rml_core::theme::color(\"--text-muted\"))"`
- 修复：`ctor: "gpui::div()"` —— 退化为普通 div，由 CSS 控制样式

**P2-6: `<deferred>` container 语义文档化**
- 文件：`translator/builtin/deferred.rs`
- 现状：文件头注释已较好说明 Deferred 非 ParentElement，但 `metadata()` 的 `.container(true)` 缺少就近注释
- 修复：在 `.container(true)` 上方添加注释说明"container=true 表示含子元素，但子元素作为构造参数传入而非 .child()"

**P3-7: is_container 保持现状**
- 决策：`TranslatorMetadata.is_container` 属设计时元数据预留字段，不删除

---

## Proposed Changes

### Step 1: E 类路径修复（9 个文件）

对以下文件执行 `super::super::component::` → `crate::compiler::setters::` 全量替换（`replace_all`）：

| 文件 | 替换次数 |
|------|----------|
| `components/tabs/gen.rs` | 3 |
| `components/tabs/tab.rs` | 3 |
| `components/tabs/setters.rs` | 7 |
| `components/tab_bar/gen.rs` | 3 |
| `components/tab_bar/setters.rs` | 3 |
| `components/table/gen.rs` | 3 |
| `components/table/column.rs` | 3 |
| `components/table/setters.rs` | 1 |
| `components/tree/gen.rs` | 3 |

**Why `crate::compiler::setters::`（绝对路径）而非 `super::setters::`：** 避免 `super::` 深度陷阱。`components/tabs/gen.rs` 距 `compiler/setters.rs` 有两级 `super`，用绝对路径最安全。

### Step 2: D 类路径修复（8 个文件）

对以下文件执行 `super::component::` → `crate::compiler::setters::` 全量替换（`replace_all`）：

| 文件 | 替换次数 |
|------|----------|
| `components/alert.rs` | 9 |
| `components/icon.rs` | 5 |
| `components/popover.rs` | 4 |
| `components/tag.rs` | 3 |
| `components/kbd.rs` | 4 |
| `components/label.rs` | 4 |
| `components/separator.rs` | 3 |
| `components/radio_group.rs` | 3 |

### Step 3: Phase 1 编译验证

```bash
cargo check -p rust-rml-engine
```

如有遗漏的路径引用，根据编译错误迭代修复。可能的遗漏：`super::component::` 在其他未被 Grep 覆盖的位置（但已全目录扫描，概率低）。

### Step 4: P0-1 修复 `<br>`

文件：`translator/builtin/br.rs`

```rust
// 第 12 行
ctor: "gpui::div().w_full().h_0()",
```

### Step 5: P0-2 修复 `<img>` bind src

文件：`translator/builtin/img.rs`

替换 `to_rust` 方法的 src 提取逻辑：

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

需新增 import：`use crate::compiler::codegen::text::gen_expr_code;`

### Step 6: P0-3 修复 `<a>` href warning

文件：`translator/builtin/a.rs`

在 `to_rust` 方法开头添加 href 检测：

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

需新增 import：`use crate::parser::ast::Attribute;`

### Step 7: P1-4 删除 is_self_closing 死字段

**7a.** `translator/builtin/meta.rs`：删除 struct 定义中的 `is_self_closing` 字段（第 25-26 行）及其文档注释（第 25 行）

**7b.** 22 个 translator 文件：删除 `is_self_closing: ...,` 赋值行（每个文件 1 行）

文件清单：anchored.rs, br.rs, a.rs, code.rs, button.rs, div.rs, h1.rs, h2.rs, h3.rs, h4.rs, h5.rs, h6.rs, input.rs, img.rs, label.rs, li.rs, p.rs, ol.rs, span.rs, svg.rs, textarea.rs, ul.rs

### Step 8: P1-5 修复 `<p>` 样式

文件：`translator/builtin/p.rs:12`

```rust
ctor: "gpui::div()",
```

### Step 9: P2-6 `<deferred>` container 语义注释

文件：`translator/builtin/deferred.rs:121`

```rust
fn metadata(&self) -> TranslatorMetadata {
    // container=true：含子元素，但子元素作为 gpui::deferred(child) 构造参数传入，
    // 非 ParentElement 的 .child() 链式调用
    TranslatorMetadata::new(TAG, DISPLAY_NAME, ComponentCategory::Layout).container(true)
}
```

### Step 10: 全量验证

```bash
# Phase 2 编译验证
cargo check -p rust-rml-engine

# 组件属性注册护栏
cargo test -p rust-rml-engine --lib props_registry::tests

# builtin 单元测试
cargo test -p rust-rml-engine --lib translator::builtin

# 全量工作区验证
cargo build --workspace
cargo test --workspace
```

---

## Assumptions & Decisions

1. **`translator/registry.rs:41` 不修改** —— `super::component::register_all` 指向 `translator::component` 模块（已验证存在 `register_all` 函数），与重命名的 `compiler::setters` 无关
2. **统一用绝对路径 `crate::compiler::setters::`** —— 避免 `super::` 深度陷阱，虽略长但最安全
3. **`is_self_closing` 删除后保留 `is_void_tag()`** —— void 标签判定单一信源，位于 `meta.rs:432`
4. **P3-7 `is_container` 保持现状** —— 属 `TranslatorMetadata` 设计时元数据完整字段集，不单独删除
5. **`<br>` 用 `w_full().h_0()`** —— GPUI 无原生换行元素，此为最接近的视觉换行近似

## Verification Steps

1. `cargo check -p rust-rml-engine` —— Phase 1 路径修复编译通过
2. `cargo test -p rust-rml-engine --lib props_registry::tests` —— 组件属性注册护栏不破坏
3. `cargo test -p rust-rml-engine --lib translator::builtin` —— builtin 单元测试通过
4. `cargo build --workspace` + `cargo test --workspace` —— 全量验证无回归

## 风险与回退

- **Phase 1 路径遗漏**：`cargo check` 迭代修复，Grep 已全目录扫描，遗漏概率低
- **Phase 2 低风险**：改动集中在 builtin/ 目录，影响面小
- **两阶段独立**：Phase 1 失败可单独回退不影响 Phase 2

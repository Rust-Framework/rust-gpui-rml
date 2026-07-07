# RML 样式属性归一化迭代计划

## Summary

将散落的 Tailwind 式样式属性（`h-flex` / `v-flex` / `gap-N` / `h-full` / `w-full` / `min-w-0` / `items-center` 等）统一归一化为 CSS 子集命名的一等直接属性（`width="full"` / `height="100px"` / `gap="8px"` / `display="flex"` / `flex-direction="column"` / `align-items="center"` 等），让 RML 的声明式样式表达收敛到单一规范的 CSS 子集命名空间。

**核心策略**：复用现有 `crates/engine/src/css/mapper.rs` 的 `map_declaration` 作为单一映射源，新增 `crates/engine/src/compiler/codegen/style_attr.rs` 模块作为 RML 直接属性 → CSS `Declaration` → GPUI 方法调用 的归一化入口，避免双轨制。

**用户已决策的两项关键澄清**：
1. 归一化覆盖范围 = 完整 CSS 子集（约 25 个属性，对齐 `mapper.rs` 已支持的全部属性）
2. 已存在的 Tailwind 式散落属性 → 全部废弃，迁移到归一化属性

---

## Current State Analysis

### RML 现有 4 条平行样式表达路径

| 路径 | 入口 | 工作状态 | 备注 |
|------|------|---------|------|
| **CSS 路径** | `class=` / `style=` / `.css` 文件 | 完整 | 经 `css::parse` → `mapper::map_declarations` |
| **直接静态属性** | `v-flex=""` / `h-flex=""` / `gap-N=""` / `h-full` / `w-full` / `min-w-0` | **破损** | `v-flex` / `h-flex` 经 `component_static_setter` 命中 `StyledExt` 方法生成 `.v_flex()`；`gap-N` / `h-full` / `w-full` / `min-w-0` 无 setter，被静默丢弃（demo 中 `<div gap-2="">` 实际未生效） |
| **窗口外壳属性** | 根标签 `width="..."` / `height="..."` | 仅根标签 | 由 `shell.rs` 处理，子元素不可用 |
| **事件** | `on-click` / `on-change` 等 | 已归一化 | 无需本迭代处理 |

### 关键文件现状

- [crates/engine/src/css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) — `map_declaration` 已支持 ~25 个 CSS 属性（width/height/padding/margin/border/font/flex/overflow/color 等），`width:100%` 特殊映射为 `w_full()`
- [crates/engine/src/css/parser.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/parser.rs) — `parse_single_value` 是内部方法（非 pub），但 `css::parse` 入口可解析完整 CSS 字符串
- [crates/engine/src/compiler/codegen/attribute.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs) — `apply_static_attr` 是原生元素属性 codegen 入口，当前未知属性输出 warning 并丢弃
- [crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) — `component_static_setter` 含 `h_flex` / `v_flex` 的 match 臂（约 486-492 行），且组件专用 setter 之前未做样式属性路由
- [crates/engine/src/compiler/props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) — `COMMON_STATIC_PROPS` 含 `"h_flex"` / `"v_flex"`
- [crates/engine/src/compiler/code_editor/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs) — CodeEditor 对 `h_full` 有特殊处理（第 64-71 行、118-124 行、207 行），需迁移
- [crates/engine/src/parser/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/mod.rs) — `normalize_attr_name` 将 kebab-case `h-full` → snake_case `h_full`，归一化后属性名按 snake_case 匹配 setter
- [demo/src/cases/common/case_doc_page.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/case_doc_page.rml) 及所有 `*-case.rml` — 大量使用 `v-flex=""` / `h-flex=""` / `gap-N=""` 待迁移

### 归一化范围（对齐 `mapper.rs` 已支持的 CSS 子集）

| 类别 | 归一化属性 | 等价 GPUI 方法 | 语义快捷词 |
|------|-----------|--------------|----------|
| **盒模型** | `width` / `height` | `w(...)` / `h(...)` | `full` = 100% → `w_full()` / `h_full()` |
| | `padding` / `padding-top/right/bottom/left` | `p/pt/pr/pb/pl(...)` | |
| | `margin` / `margin-top/right/bottom/left` | `m/mt/mr/mb/ml(...)` | |
| | `border-radius` | `rounded(...)` | |
| | `border` / `border-color` / `border-top/right/bottom/left` | `border_N()` / `border_color(...)` | |
| **文本** | `font-size` | `text_size(...)` | |
| | `font-weight` | `font_weight(...)` | `bold` / `normal` / `medium` 等关键字 |
| | `font-family` | `font_family(...)` | |
| | `text-align` | `text_left/center/right()` | |
| | `line-height` | `line_height(...)` | |
| | `white-space` | `whitespace_normal/nowrap()` | `nowrap` / `pre` → `whitespace_nowrap()` |
| | `color` | `text_color(...)` | 支持 `var(--name)` 运行时主题查询 |
| | `background` / `background-color` | `bg(...)` | 支持 `var(--name)` 运行时主题查询 |
| **Flexbox** | `display` | `flex()` / `hidden()` | `flex` / `none` |
| | `flex-direction` | `flex_row()` / `flex_col()` | `row` / `column` |
| | `flex-wrap` | `flex_wrap()` / `flex_nowrap()` | `wrap` / `nowrap` |
| | `justify-content` | `justify_center/start/end/between()` | `center` / `flex-start` / `flex-end` / `space-between` |
| | `align-items` | `items_center/start/end/stretch()` | `center` / `flex-start` / `flex-end` / `stretch` |
| | `flex` | `flex_grow(N).flex_shrink_0().flex_basis(px(0))` | 数字（如 `flex="1"`） |
| | `gap` | `gap(...)` | |
| | `min-width` / `max-width` / `min-height` / `max-height` | `min_w/max_w/min_h/max_h(...)` | `0` 特殊映射为 `min_w_0()` 等 |
| **视觉效果** | `opacity` | `opacity(N)` | |
| | `overflow` / `overflow-x` / `overflow-y` | `overflow_hidden/scroll/x_scrollbar/y_scrollbar()` | `hidden` / `scroll` / `auto` |

### 废弃映射表（Tailwind 式散落属性 → 归一化属性）

| 旧属性 | 新属性（等价替换） |
|--------|-----------------|
| `v-flex=""` | `display="flex" flex-direction="column"` |
| `h-flex=""` | `display="flex" flex-direction="row"` |
| `gap-2=""` | `gap="8px"`（N×4px） |
| `gap-4=""` | `gap="16px"` |
| `gap-6=""` | `gap="24px"` |
| `h-full=""` | `height="full"` |
| `w-full=""` | `width="full"` |
| `min-w-0=""` | `min-width="0"` |
| `min-h-0=""` | `min-height="0"` |
| `items-center=""` | `align-items="center"` |
| `flex-wrap=""` | `flex-wrap="wrap"` |

---

## Proposed Changes

### Step 1：创建 `style_attr.rs` 模块（独立、可单测）

**新建文件**：[crates/engine/src/compiler/codegen/style_attr.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs)

**职责**：将 RML 直接属性 `(name, value)` 转换为 GPUI 方法调用代码字符串，复用 `css::mapper::map_declaration` 作为单一映射源。

**核心 API**：

```rust
//! RML 归一化样式属性 → GPUI 方法调用
//!
//! 将 RML 直接属性（如 `width="full"` / `gap="8px"` / `display="flex"`）
//! 转换为 GPUI `Styled` trait 方法调用代码字符串。
//!
//! ## 设计
//!
//! 复用 `css::mapper::map_declaration` 作为单一映射源，避免双轨制。
//! RML 直接属性 → 构造 CSS `Declaration` → `map_declaration` → GPUI 方法。
//!
//! ## 语义快捷词
//!
//! `width="full"` / `height="full"` 等价于 `width="100%"` / `height="100%"`，
//! 最终生成 `.w_full()` / `.h_full()`。

use crate::css::{self, ast::{Declaration, Value, Unit}};

/// 判断属性名是否为归一化样式属性
///
/// 入口参数 `name` 为 normalize 后的 snake_case 形式（如 `flex_direction`），
/// 内部转回 kebab-case 与 `mapper.rs` 的 CSS 属性名匹配。
pub fn is_style_attr(name: &str) -> bool {
    let kebab = name.replace('_', "-");
    matches!(kebab.as_str(),
        // 盒模型
        "width" | "height" |
        "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" |
        "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" |
        "border-radius" |
        "border" | "border-color" | "border-top" | "border-right" | "border-bottom" | "border-left" |
        // 文本
        "font-size" | "font-weight" | "font-family" |
        "text-align" | "line-height" | "white-space" |
        "color" | "background" | "background-color" |
        // Flexbox
        "display" | "flex-direction" | "flex-wrap" |
        "justify-content" | "align-items" | "flex" | "gap" |
        "min-width" | "max-width" | "min-height" | "max-height" |
        // 视觉效果
        "opacity" | "overflow" | "overflow-x" | "overflow-y"
    )
}

/// 应用样式属性，返回 GPUI 方法调用代码（含前导 `.`）
///
/// 如 `apply_style_attr("width", "full")` → `Some(".w_full()")`
/// 如 `apply_style_attr("gap", "8px")` → `Some(".gap(gpui::px(8.0))")`
/// 如 `apply_style_attr("display", "flex")` → `Some(".flex()")`
///
/// 不支持的值返回 `None`（调用方输出 warning）。
pub fn apply_style_attr(name: &str, value: &str) -> Option<String> {
    let kebab = name.replace('_', "-");
    let css_value = parse_rml_value(value)?;
    let decl = Declaration { property: kebab, value: css_value };
    let mapped = css::mapper::map_declarations(&[decl], &Default::default());
    if mapped.is_empty() {
        None
    } else {
        Some(mapped)
    }
}

/// 将 RML 属性值字符串解析为 CSS `Value`
///
/// 语义快捷词：
/// - `full` → `Value::Length(100.0, Unit::Percent)`（特殊映射为 `w_full()` / `h_full()`）
/// - `0` → `Value::Number(0.0)`（min-width/min-height=0 特殊映射为 `min_w_0()` / `min_h_0()`）
///
/// 其他值委托 `css::parse` 解析单个声明值。
fn parse_rml_value(s: &str) -> Option<Value> {
    let trimmed = s.trim();
    if trimmed == "full" {
        return Some(Value::Length(100.0, Unit::Percent));
    }
    // 用 css::parse 解析 `prop: value;` 形式，取首条声明的 value
    let fake = format!("* {{ tmp: {}; }}", trimmed);
    let sheet = css::parse(&fake).ok()?;
    sheet.rules.into_iter().next()
        ?.declarations.into_iter().next()
        .map(|d| d.value)
}
```

**单元测试要点**：
- `is_style_attr("width")` / `is_style_attr("flex_direction")` 返回 `true`
- `is_style_attr("label")` / `is_style_attr("h_flex")` 返回 `false`
- `apply_style_attr("width", "full")` → `".w_full()"`
- `apply_style_attr("width", "100px")` → `".w(gpui::px(100.0))"`
- `apply_style_attr("width", "50%")` → `".w(gpui::relative(0.5))"`
- `apply_style_attr("height", "full")` → `".h_full()"`
- `apply_style_attr("gap", "8px")` → `".gap(gpui::px(8.0))"`
- `apply_style_attr("display", "flex")` → `".flex()"`
- `apply_style_attr("flex_direction", "column")` → `".flex_col()"`
- `apply_style_attr("align_items", "center")` → `".items_center()"`
- `apply_style_attr("min_width", "0")` → `".min_w_0()"`
- `apply_style_attr("color", "var(--text-color)")` → `".text_color(rml::theme::color(\"--text-color\"))"`
- `apply_style_attr("width", "invalid!")` → `None`（解析失败）

**新增模块声明**：在 [crates/engine/src/compiler/codegen/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs) 加入 `pub mod style_attr;`（如已存在 `mod.rs` 则追加声明）。

### Step 2：接入 codegen 流程

**修改** [crates/engine/src/compiler/codegen/attribute.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs) 的 `apply_static_attr`：

```rust
pub(super) fn apply_static_attr(name: &str, value: &str) -> String {
    match name {
        "class" | "id" => String::new(),
        "ref" => String::new(),
        "style" => apply_inline_style(value),
        "src" | "href" => String::new(),
        "type" => String::new(),
        _ => {
            // 归一化样式属性：复用 css::mapper 单一映射源
            if let Some(s) = super::style_attr::apply_style_attr(name, value) {
                return s;
            }
            eprintln!(
                "[rml warning] unknown static attribute `{}` (value={:?}) on native element; \
                 property will be dropped. Register it in props_registry or add a match arm.",
                name, value
            );
            String::new()
        }
    }
}
```

**修改** `apply_bind_attr`：对 `is_style_attr(name)` 为真的 bind 形式（如 `width={computed}`）输出 warning 并丢弃（运行时动态样式仍走 `class=` + 主题切换）。

**修改** [crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 的 `component_static_setter`：

在所有组件专用 setter 之前（即 match 臂顶部，紧跟现有 `super::tooltip::static_setter(...)` 之后）插入：

```rust
// 归一化样式属性：对所有扩展组件生效（gpui-component 实现 Styled trait）
if let Some(s) = super::codegen::style_attr::apply_style_attr(name, value) {
    return Some(s);
}
```

**位置选择理由**：组件专用 setter（如 TabBar 的 `bordered` 是组件语义属性，生成 `.bordered(bool)`）与 CSS `border` 属性名不同；`variant`、`selected_index`、`open` 等也不与 CSS 样式属性同名。归一化样式属性放置在组件专用 setter 之前是安全的，因为它们的属性名空间不重叠。

### Step 3：注册样式属性到 props_registry

**修改** [crates/engine/src/compiler/props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)：

新增 `STYLE_ATTR_PROPS` 常量，列出所有归一化样式属性（snake_case 形式），并在 `is_prop_registered` 中合并查询：

```rust
/// 归一化样式属性（对所有元素与组件生效，由 `style_attr::apply_style_attr` 处理）
///
/// 列表对齐 `css/mapper.rs` 支持的 CSS 子集。
pub const STYLE_ATTR_PROPS: &[&str] = &[
    // 盒模型
    "width", "height",
    "padding", "padding_top", "padding_right", "padding_bottom", "padding_left",
    "margin", "margin_top", "margin_right", "margin_bottom", "margin_left",
    "border_radius",
    "border", "border_color", "border_top", "border_right", "border_bottom", "border_left",
    // 文本
    "font_size", "font_weight", "font_family",
    "text_align", "line_height", "white_space",
    "color", "background", "background_color",
    // Flexbox
    "display", "flex_direction", "flex_wrap",
    "justify_content", "align_items", "flex", "gap",
    "min_width", "max_width", "min_height", "max_height",
    // 视觉效果
    "opacity", "overflow", "overflow_x", "overflow_y",
];
```

`is_prop_registered` 在通用属性与组件专用属性查询之前加入 `STYLE_ATTR_PROPS.contains(&attr)` 判断。

### Step 4：废弃 Tailwind 式散落属性

**修改** [crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)：

1. 移除 `component_static_setter` 中 `h_flex` / `v_flex` 的 match 臂（约 486-492 行）
2. 在 `apply_static_attr`（attribute.rs）的 match 臂中新增 deprecation warning：
   ```rust
   "h_flex" | "v_flex" | "h_full" | "w_full" | "min_w_0" | "min_h_0" => {
       eprintln!(
           "[rml deprecation] `{}` is deprecated; use normalized CSS attribute instead \
            (e.g. display=\"flex\" flex-direction=\"row\" for h-flex, width=\"full\" for w-full)",
           name
       );
       String::new()
   }
   ```
3. 删除 [crates/engine/src/compiler/props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) 中 `COMMON_STATIC_PROPS` 的 `"h_flex"` / `"v_flex"` 条目

**修改** [crates/engine/src/compiler/code_editor/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs)：

1. 移除第 64-71 行的 `h_full` 静态属性读取
2. 修改第 118-124 行的 `height_chain`，默认使用 `.h(gpui::px(360.))`，不再特殊处理 `h_full`
3. 移除第 207 行 `is_handled_inline` 列表中的 `"h_full"`

**用户迁移路径**：用户原写 `<CodeEditor h-full="" />` 改为 `<CodeEditor height="full" />`（`apply_style_attr` 会生成 `.h_full()`，在 style_chain 后写覆盖默认 `.h(360px)`，GPUI 后写覆盖前写的语义成立）。

### Step 5：迁移所有 demo `.rml` 文件

**修改** [demo/src/cases/common/case_doc_page.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/case_doc_page.rml) 及所有 `demo/src/cases/*-case.rml`、`demo/src/lsp/code_editor_tab.rml`：

迁移规则（依废弃映射表）：

| 旧 | 新 |
|----|---|
| `<div v-flex="" class="case-pane doc-pane">` | `<div display="flex" flex-direction="column" class="case-pane doc-pane">` |
| `<div v-flex="" gap-2="">` | `<div display="flex" flex-direction="column" gap="8px">` |
| `<div h-flex="" gap-4="">` | `<div display="flex" flex-direction="row" gap="16px">` |
| `<div h-flex="" gap-4="" items-center="">` | `<div display="flex" flex-direction="row" gap="16px" align-items="center">` |
| `<div h-flex="" gap-2="" flex-wrap="">` | `<div display="flex" flex-direction="row" gap="8px" flex-wrap="wrap">` |
| `<CodeEditor h-full="" context-menu="..." />` | `<CodeEditor height="full" context-menu="..." />` |

**优化考虑**：可考虑在 [demo/assets/styles.css](file:///d:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css) 中新增 `.v-flex` / `.h-flex` 工具类（仅 demo 用，框架不内置），让 demo 案例文件保持简洁。但**不在本迭代强制要求**，按"全部迁移到归一化属性"用户决策执行。

### Step 6：更新文档

**修改** `.trae/skills/rml-component/07-size-layout-conventions.md`：
- 废弃 `h-flex` / `v-flex` / `gap-N` 章节
- 新增"归一化样式属性"章节，列出完整 CSS 子集与语义快捷词
- 添加迁移示例

**修改** `.trae/skills/rml-component/03-property-classification.md`：
- 在属性分类中新增"样式属性"分类（独立于 static/bind/event）
- 说明样式属性同时支持原生元素与扩展组件

**新建** `.trae/skills/rml-component/migration-style-normalization.md`：
- 迁移指南：旧 Tailwind 式属性 → 归一化属性的完整映射表
- deprecation warning 触发条件
- CodeEditor `h-full` 迁移说明

---

## Assumptions & Decisions

### 已确认决策

1. **归一化范围**：完整 CSS 子集（约 25 个属性），对齐 `mapper.rs` 已支持的全部属性
2. **迁移策略**：全部废弃 Tailwind 式散落属性，迁移到归一化属性
3. **保留项**：`font_bold` / `font_semibold` 等 `StyledExt` 字体权重快捷方法保留（用户决策），不强制迁移到 `font-weight="bold"`
4. **事件系统不动**：事件已归一化（`on-click` / `on-change`），本迭代不涉及

### 关键假设

1. **属性名冲突**：组件专用属性（`bordered` / `variant` / `selected_index` / `open` / `title` 等）与 CSS 样式属性名不重叠，`style_attr` 检查可安全放置在组件专用 setter 之前
2. **CodeEditor 默认高度**：移除 `h_full` 特殊处理后，默认 `.h(360px)` 仍生效；用户写 `height="full"` 会在 style_chain 末尾追加 `.h_full()`，GPUI 后写覆盖前写
3. **bind 形式样式属性不支持**：当前不支持 `width={computed}` 形式（运行时动态样式仍走 `class=` + 主题切换），输出 warning
4. **CSS 变量保留运行时查询**：`color="var(--text-color)"` 仍生成 `rml::theme::color("--text-color")` 调用，主题切换即时生效

### 不在本迭代范围

- bind 形式样式属性（如 `width={computed}`）支持
- 新增 CSS 属性（如 `box-shadow` / `transform` 等 `mapper.rs` 未支持的）
- 用户自定义 CSS 工具类（`.v-flex` / `.h-flex`）的内置支持

---

## Verification Steps

### Step 1 完成后

```bash
cargo test -p rust-rml-engine style_attr
```

验证：`is_style_attr` / `apply_style_attr` 单元测试全部通过。

### Step 2-4 完成后

```bash
cargo build -p rust-rml-engine
cargo test -p rust-rml-engine
```

验证：
- 编译通过
- 现有 `component_static_setter` / `apply_static_attr` 测试不回归
- `h_flex` / `v_flex` 测试已移除或改为验证 deprecation warning

### Step 5 完成后

```bash
cargo build -p demo
```

验证：demo 编译通过，无 `[rml warning]` 输出。

### 全部完成后

```bash
cargo test -p rust-rml-engine
cargo build -p demo
cargo run -p demo
```

验证：
- 所有测试通过
- demo 启动后样式与迁移前一致（v-flex/h-flex 布局、gap 间距、CodeEditor 全高）
- 控制台无 deprecation warning 输出

---

## 实施顺序

1. **Step 1** → 创建 `style_attr.rs` 模块（独立、可单测）
2. **Step 2** → 接入 codegen 流程（`attribute.rs` + `component.rs`）
3. **Step 3** → 注册到 `props_registry.rs`
4. **Step 4** → 废弃 `h_flex` / `v_flex`，修复 CodeEditor `h_full` 特殊处理
5. **Step 5** → 迁移所有 demo `.rml` 文件
6. **Step 6** → 更新文档
7. **完整测试**：`cargo test -p rust-rml-engine` + `cargo build -p demo` + `cargo run -p demo`

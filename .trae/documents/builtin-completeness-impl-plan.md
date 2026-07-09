# Builtin 节点完备性实施计划

## Summary

确保 `crates/engine/src/compiler/translator/builtin` 中的基本节点（对标原生 HTML 的 GPUI 原语）的**原始节点、属性、样式**支持完备，达成评判标准：**可基于 `.rml` + `.css` 开发高级组件，可基于这些基础元素构建 gpui-component 所有组件的完整能力**。

本计划基于对 GPUI `Styled` trait 全量方法清单、GPUI 原语元素（`anchored`/`deferred`）、以及当前 RML mapper/style_attr/attribute 代码的实际审计，补齐三块缺口：
1. **CSS 属性映射缺口**（mapper.rs + style_attr.rs 白名单同步）
2. **关键正确性 Bug**（overflow-x/y 生成不存在的 GPUI 方法名）
3. **原语元素缺口**（`<anchored>` / `<deferred>`，覆盖层组件 Popover/Tooltip/Dropdown 必需）

## Current State Analysis

### 已完成（Task #1/#2 from prior session，经代码验证）
- **mapper.rs P0 样式**：定位（position/top/right/bottom/left/inset）、阴影（box-shadow 9 档）、cursor（23 种）、visibility、text-overflow、line-clamp、truncate ✅
- **Flexbox 补全**：flex-direction（4 种）、flex-wrap（3 种）、justify-content（7 种含 space-around/evenly）、align-items（5 种含 baseline）✅
- **原语元素**：`<img>` 真实化为 `gpui::img(src)`（动态 ctor）、`<svg>` 新增映射 `gpui::svg()`（path setter）✅
- **meta.rs**：`builtin_engine::translate` 已支持动态 `ctor: &str` 参数 ✅

### 缺口 1：CSS 属性映射缺失（mapper.rs）
经审计 GPUI `styled.rs` + `gpui_macros/styles.rs` 全量方法，以下 GPUI 原生方法**尚未映射**对应 CSS 属性：

| CSS 属性 | GPUI 方法 | 状态 |
|---|---|---|
| `align-self` | `self_start/self_end/self_flex_start/self_flex_end/self_center/self_baseline/self_stretch` | ❌ 缺失 |
| `align-content` | `content_normal/content_center/content_start/content_end/content_between/content_around/content_evenly/content_stretch` | ❌ 缺失 |
| `font-style` | `italic()/not_italic()` | ❌ 缺失 |
| `text-decoration` | `underline()/line_through()/text_decoration_none()` | ❌ 缺失 |
| `border-style: dashed` | `border_dashed()` | ❌ 缺失 |
| `border-x` / `border-y` | `border_x_N()/border_y_N()`（宏生成） | ❌ 缺失 |
| `border-top-left-radius` 等 4 角 | `rounded_tl/tr/bl/br()` | ❌ 缺失 |
| `aspect-ratio` | `aspect_ratio(f32)/aspect_square()` | ❌ 缺失 |
| `flex-grow` / `flex-shrink` / `flex-basis` | `flex_grow()/flex_shrink()/flex_basis()` | ❌ 缺失（仅有 `flex: <n>` 简写） |
| `display: block` / `display: grid` | `block()/grid()` | ❌ 缺失（仅有 flex/none） |
| `grid-template-columns/rows` | `grid_cols(u16)/grid_rows(u16)` | ❌ 缺失 |
| `grid-column` / `grid-row`（span） | `col_span()/row_span()` | ❌ 缺失 |
| `grid-column-start/end` / `grid-row-start/end` | `col_start/col_end/row_start/row_end` | ❌ 缺失 |

### 缺口 2：关键正确性 Bug（mapper.rs overflow-x/y）
**当前代码生成不存在的 GPUI 方法，是潜在编译错误**（被字符串匹配单测掩盖）：

| CSS | 当前生成 | 实际 GPUI 方法 | 问题 |
|---|---|---|---|
| `overflow-x: scroll` | `.overflow_x_scrollbar()` | `.overflow_x_scroll()` | ❌ 方法不存在 |
| `overflow-y: scroll` | `.overflow_y_scrollbar()` | `.overflow_y_scroll()` | ❌ 方法不存在 |
| `overflow-x: hidden` | `.overflow_hidden()` | `.overflow_x_hidden()` | ❌ 误设双轴 |
| `overflow-y: hidden` | `.overflow_hidden()` | `.overflow_y_hidden()` | ❌ 误设双轴 |

已确认：`overflow_x_scrollbar`/`overflow_y_scrollbar` 在 gpui crates 与本仓库 crates 中均**不存在**（grep 零命中）；GPUI 原生方法为 `overflow_x_scroll`/`overflow_y_scroll`（定义于 `gpui/src/elements/div.rs:1318/1324`，对 `Div` 可用，所有 builtin 容器均映射为 `gpui::div()` 故可用）。

### 缺口 3：原语元素缺失（builtin/）
要构建 gpui-component 的 **Popover/Tooltip/Dropdown/Modal** 等覆盖层组件，必须能从基础元素表达"锚定定位"与"延后绘制（z-order）"。GPUI 无 `z_index`（已确认 styled.rs 与宏中均无），z-order 通过 `Deferred` 元素实现；定位通过 `Anchored` 元素实现。当前 builtin 缺这两个原语：
- `<anchored>` → `gpui::anchored()`：无参构造，`ParentElement`（可含子），**非 Styled**（CSS 不直接生效，需作用于内部 div）。setter：`.anchor(Anchor)`/`.position()`/`.offset()`/`.snap_to_window()`。
- `<deferred>` → `gpui::deferred(child)`：**child 必须作为构造参数**（非 builder），`with_priority(usize)`/`priority(usize)` 控制 z-order，非 Styled 非 ParentElement。

### 现有 builtin 元素现状（已读全部 22 个文件）
- 容器类（div/span/p/h1-h6/button/ul/ol/li/a/label/code）→ `gpui::div()`，走 `BuiltinTranslator` 引擎 ✅
- `img` → `gpui::img(src)`（自定义 to_rust 提取 src 为 ctor 参数）✅
- `svg` → `gpui::svg()`（path 走 attribute.rs `.path()` setter）✅
- `br` → `gpui::div().hidden()` ✅
- `input`/`textarea` → 有 `model={field}` 时生成 gpui-component TextInput；否则退化为 div ✅
- `a` → `gpui::div()`，`href` 在 attribute.rs 中静默丢弃（`"src" | "href" => String::new()`）—— 链接行为需用户加 `on-click`，可接受，不改

### GPUI 能力边界（已确认）
- **无 `z_index`**：z-order 经 `deferred().with_priority(N)` 实现
- **Anchored/Deferred 非 Styled**：CSS 须作用于内部 div，不能直接作用于 anchored/deferred
- **position 仅 relative/absolute**：无 static/fixed（宏仅生成 relative/absolute）
- **overflow scroll 方法在 Div 上**：`overflow_x_scroll`/`overflow_y_scroll`/`overflow_scroll`（非泛 Styled trait，但 div 可用）

## Proposed Changes

### Task 1【P0 正确性】修复 overflow-x/y 错误方法名
**文件**：`crates/engine/src/css/mapper.rs`

修改 `map_declaration` 中 `overflow-x` / `overflow-y` 两个 match arm（位于"视觉效果"段）：

```rust
"overflow-x" => match &value {
    Value::Keyword(k) if k == "hidden" => Some("overflow_x_hidden()".into()),
    Value::Keyword(k) if k == "scroll" || k == "auto" => Some("overflow_x_scroll()".into()),
    _ => None,
},
"overflow-y" => match &value {
    Value::Keyword(k) if k == "hidden" => Some("overflow_y_hidden()".into()),
    Value::Keyword(k) if k == "scroll" || k == "auto" => Some("overflow_y_scroll()".into()),
    _ => None,
},
```

**同步修改受影响单测**（mapper.rs `tests` 模块 + style_attr.rs `tests` 模块）：
- `map_overflow_x_scroll`：断言由 `.overflow_x_scrollbar()` 改为 `.overflow_x_scroll()`
- `apply_overflow_y_scroll`：断言由 `.overflow_y_scrollbar()` 改为 `.overflow_y_scroll()`
- `apply_overflow_x_auto`：断言由 `.overflow_x_scrollbar()` 改为 `.overflow_x_scroll()`
- 新增 `map_overflow_x_hidden` / `apply_overflow_y_hidden` 验证单轴 hidden 不污染另一轴

### Task 2【P1 样式】补齐 typography / flexbox / border / 尺寸映射
**文件**：`crates/engine/src/css/mapper.rs`（在 `truncate` arm 之后、`_ => None` 之前插入新 arm）；`crates/engine/src/compiler/codegen/style_attr.rs`（扩展 `is_style_attr` 白名单）

mapper.rs 新增 match arm（精确代码）：

```rust
// ─── display 扩展（修改现有 display arm，新增 block/grid）───
// 原: "display" => match &value { flex=>flex(), none=>hidden(), _=>None }
// 改为:
"display" => match &value {
    Value::Keyword(k) if k == "flex" => Some("flex()".into()),
    Value::Keyword(k) if k == "block" => Some("block()".into()),
    Value::Keyword(k) if k == "grid" => Some("grid()".into()),
    Value::Keyword(k) if k == "none" => Some("hidden()".into()),
    _ => None,
},

// ─── 文本装饰 ───
"text-decoration" => match &value {
    Value::Keyword(k) => match k.as_str() {
        "underline" => Some("underline()".into()),
        "line-through" => Some("line_through()".into()),
        "none" => Some("text_decoration_none()".into()),
        _ => None,
    },
    _ => None,
},
// ─── 字体风格 ───
"font-style" => match &value {
    Value::Keyword(k) => match k.as_str() {
        "italic" => Some("italic()".into()),
        "normal" => Some("not_italic()".into()),
        _ => None,
    },
    _ => None,
},
// ─── align-self ───
"align-self" => match &value {
    Value::Keyword(k) => match k.as_str() {
        "start" => Some("self_start()".into()),
        "flex-start" => Some("self_flex_start()".into()),
        "end" => Some("self_end()".into()),
        "flex-end" => Some("self_flex_end()".into()),
        "center" => Some("self_center()".into()),
        "stretch" => Some("self_stretch()".into()),
        "baseline" => Some("self_baseline()".into()),
        _ => None,  // auto = 默认，跳过
    },
    _ => None,
},
// ─── align-content ───
"align-content" => match &value {
    Value::Keyword(k) => match k.as_str() {
        "normal" => Some("content_normal()".into()),
        "center" => Some("content_center()".into()),
        "start" | "flex-start" => Some("content_start()".into()),
        "end" | "flex-end" => Some("content_end()".into()),
        "space-between" => Some("content_between()".into()),
        "space-around" => Some("content_around()".into()),
        "space-evenly" => Some("content_evenly()".into()),
        "stretch" => Some("content_stretch()".into()),
        _ => None,
    },
    _ => None,
},
// ─── border 细化 ───
"border-x" => shorthand_border(&value, vars, "x"),
"border-y" => shorthand_border(&value, vars, "y"),
"border-style" => match &value {
    Value::Keyword(k) if k == "dashed" => Some("border_dashed()".into()),
    _ => None,  // solid/none/dotted 为默认或 GPUI 不支持，跳过
},
// ─── 圆角细化（4 角）───
"border-top-left-radius" => length_method("rounded_tl", &value),
"border-top-right-radius" => length_method("rounded_tr", &value),
"border-bottom-right-radius" => length_method("rounded_br", &value),
"border-bottom-left-radius" => length_method("rounded_bl", &value),
// ─── flex 分项 ───
"flex-grow" => match &value {
    Value::Number(n) => Some(format!("flex_grow({:?})", n)),
    _ => None,
},
"flex-shrink" => match &value {
    Value::Number(n) => Some(format!("flex_shrink({:?})", n)),
    _ => None,
},
"flex-basis" => length_or_percentage_method("flex_basis", &value),
// ─── aspect-ratio ───
"aspect-ratio" => match &value {
    Value::Keyword(k) if k == "square" => Some("aspect_square()".into()),
    Value::Number(n) => Some(format!("aspect_ratio({:?})", n)),
    _ => None,  // "16/9" 比式字符串暂不支持，文档化
},
```

style_attr.rs `is_style_attr` 白名单新增（kebab 形式）：
`"display"` 已有；新增 `"text-decoration" | "font-style" | "align-self" | "align-content" | "border-x" | "border-y" | "border-style" | "border-top-left-radius" | "border-top-right-radius" | "border-bottom-right-radius" | "border-bottom-left-radius" | "flex-grow" | "flex-shrink" | "flex-basis" | "aspect-ratio"`。

**单测**（mapper.rs + style_attr.rs 各加）：`map_text_decoration_underline`、`map_font_style_italic`、`map_align_self_center`、`map_align_content_between`、`map_border_x_shorthand`、`map_border_style_dashed`、`map_border_top_left_radius`、`map_aspect_ratio_square`、`map_display_block`、`map_display_grid`、`map_flex_grow`，及对应 `apply_*` 用例。

### Task 3【P2 样式】CSS Grid 映射
**文件**：mapper.rs + style_attr.rs

mapper.rs 新增（`aspect-ratio` arm 之后）：

```rust
// ─── CSS Grid ───
"grid-template-columns" => match &value {
    Value::Number(n) => Some(format!("grid_cols({}u16)", *n as u16)),
    _ => None,
},
"grid-template-rows" => match &value {
    Value::Number(n) => Some(format!("grid_rows({}u16)", *n as u16)),
    _ => None,
},
"grid-column" => match &value {
    // grid-column: span <N>
    Value::List(items) if items.len() == 2 => {
        if let (Value::Keyword(k), Value::Number(n)) = (&items[0], &items[1]) {
            if k == "span" { return Some(format!("col_span({}u16)", *n as u16)); }
        }
        None
    }
    _ => None,
},
"grid-row" => match &value {
    Value::List(items) if items.len() == 2 => {
        if let (Value::Keyword(k), Value::Number(n)) = (&items[0], &items[1]) {
            if k == "span" { return Some(format!("row_span({}u16)", *n as u16)); }
        }
        None
    }
    _ => None,
},
"grid-column-start" => match &value {
    Value::Number(n) => Some(format!("col_start({}i16)", *n as i16)),
    _ => None,
},
"grid-column-end" => match &value {
    Value::Number(n) => Some(format!("col_end({}i16)", *n as i16)),
    _ => None,
},
"grid-row-start" => match &value {
    Value::Number(n) => Some(format!("row_start({}i16)", *n as i16)),
    _ => None,
},
"grid-row-end" => match &value {
    Value::Number(n) => Some(format!("row_end({}i16)", *n as i16)),
    _ => None,
},
```

style_attr.rs 白名单新增：`"grid-template-columns" | "grid-template-rows" | "grid-column" | "grid-row" | "grid-column-start" | "grid-column-end" | "grid-row-start" | "grid-row-end"`。

**单测**：`map_grid_template_columns`、`map_grid_column_span`、`map_grid_column_start` 等。

### Task 4【P1 元素】新增 `<anchored>` 原语
**新文件**：`crates/engine/src/compiler/translator/builtin/anchored.rs`

映射 `gpui::anchored()`（无参构造，ParentElement 容器，**非 Styled**）。特殊属性 `anchor` → `.anchor(gpui::Anchor::X)`。

```rust
//! `<anchored>` translator —— 映射到 GPUI 原生 `gpui::anchored()`
//!
//! Anchored 用于锚定定位（Popover/Tooltip/Dropdown 基础），非 Styled，
//! CSS 须作用于内部 div。anchor 属性设置锚定角落。

use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

const META: &BuiltinMeta = &BuiltinMeta {
    tag: "anchored",
    display_name: "Anchored",
    category: ComponentCategory::Primitive,
    ctor: "gpui::anchored()",
    is_container: true,
    is_self_closing: false,
    is_styled: false,  // 新增字段，见 Task 4b
};

#[derive(Debug)]
pub struct AnchoredTranslator;

impl IRmlTranslator for AnchoredTranslator {
    fn tag(&self) -> &'static str { META.tag }
    fn to_rust(&self, elem: &Element, ctx: &CodegenCtx, id_counter: &mut usize,
               loop_vars: &[String], parents: &[ParentInfo]) -> Result<(String, bool), CodegenError> {
        super::meta::builtin_engine::translate(elem, ctx, id_counter, loop_vars, parents, META.ctor, META.is_styled)
    }
    fn to_rml(&self, elem: &Element, ctx: &super::PrinterCtx) -> Result<String, super::PrintError> {
        BuiltinTranslator { meta: META }.to_rml(elem, ctx)
    }
    fn metadata(&self) -> super::TranslatorMetadata { META.to_metadata() }
}
```

**Task 4b：BuiltinMeta 增加 `is_styled: bool` 字段**
- `meta.rs`：`BuiltinMeta` 结构体新增 `pub is_styled: bool`；所有现有元素 META 初始化补 `is_styled: true`；`translate` 签名新增 `is_styled: bool` 参数
- 引擎 `translate` 内：
  - CSS 块 `if let Some(sheet) = ...` 增加条件 `if is_styled &&`
  - 属性循环 `Attribute::Static` 分支：`if !is_styled && style_attr::is_style_attr(name) { continue; }`（非 Styled 元素跳过样式属性，避免生成不存在方法）
  - `BuiltinTranslator::to_rust` 调用改为 `translate(..., self.meta.ctor, self.meta.is_styled)`
  - `img.rs` 调用改为 `translate(..., &ctor, true)`（img 是 Styled）

**Task 4c：attribute.rs 新增 anchor setter**
```rust
// anchored 专用：anchor 设置锚定角落
"anchor" => match value {
    "top-left" => ".anchor(gpui::Anchor::TopLeft)".to_string(),
    "top-right" => ".anchor(gpui::Anchor::TopRight)".to_string(),
    "bottom-left" => ".anchor(gpui::Anchor::BottomLeft)".to_string(),
    "bottom-right" => ".anchor(gpui::Anchor::BottomRight)".to_string(),
    _ => String::new(),
},
```
（实现时需确认 `gpui::Anchor` 枚举变体名，若不同则按实际调整。）

**Task 4d：mod.rs 注册**
- `pub mod anchored;`
- `register_all` 中 `registry.register(anchored::AnchoredTranslator);`

### Task 5【P1 元素】新增 `<deferred>` 原语
**新文件**：`crates/engine/src/compiler/translator/builtin/deferred.rs`

`gpui::deferred(child)` 要求 child 作为构造参数（非 builder，非 ParentElement）。采用**自定义 to_rust**（不走引擎的 `.child()` 链），取单一子元素作为 ctor 参数，可选 `priority` 属性 → `.with_priority(N)`。

```rust
//! `<deferred>` translator —— 映射到 GPUI 原生 `gpui::deferred(child)`
//!
//! Deferred 延后绘制以实现 z-order（GPUI 无 z_index）。child 必须作为构造参数，
//! 故采用自定义 to_rust：取单一子元素代码作为 ctor 参数。priority 属性控制层级。
//! 注意：deferred 上的 if/show/each 指令不直接支持（请作用于内部元素）。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::node::gen_node_impl;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};

const TAG: &str = "deferred";

#[derive(Debug)]
pub struct DeferredTranslator;

impl IRmlTranslator for DeferredTranslator {
    fn tag(&self) -> &'static str { TAG }

    fn to_rust(&self, elem: &Element, ctx: &CodegenCtx, id_counter: &mut usize,
               loop_vars: &[String], parents: &[ParentInfo]) -> Result<(String, bool), CodegenError> {
        // 取可选 priority 静态属性
        let priority = elem.attributes.iter().find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "priority" => value.parse::<usize>().ok(),
            _ => None,
        });
        // 取单一子元素（跳过纯空白文本节点）
        let child = elem.children.iter().find_map(|c| match c {
            Node::Element(e) => Some(e),
            _ => None,
        }).ok_or_else(|| CodegenError {
            message: "`<deferred>` 必须包含恰好一个子元素".to_string(),
            span: Some(elem.span),
        })?;
        let (child_code, is_iter) = gen_node_impl(&Node::Element(child.clone()), ctx, 0, id_counter, loop_vars, parents)?;
        if is_iter {
            return Err(CodegenError {
                message: "`<deferred>` 子元素不支持 `each` 指令".to_string(),
                span: Some(elem.span),
            });
        }
        let mut code = format!("gpui::deferred({})", child_code);
        if let Some(p) = priority {
            code.push_str(&format!(".with_priority({})", p));
        }
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, super::PrintError> {
        super::meta::builtin_engine::print(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new(TAG, "Deferred", ComponentCategory::Primitive).container(true)
    }
}
```

**mod.rs 注册**：`pub mod deferred;` + `registry.register(deferred::DeferredTranslator);`

**注意**：`gen_node_impl` 签名需确认（接收 `&Node` 还是 `&Element`）；若接收 `&Node` 则传 `&Node::Element(child.clone())`，若接收 `&Element` 则传 `child`。实现时按实际签名调整。

### Task 6【文档化】GPUI 能力边界
在 `mapper.rs` 顶部模块注释补充"GPUI 能力边界"小节：
- 无 `z_index`：z-order 经 `<deferred priority="N">` 实现
- position 仅 `relative`/`absolute`（无 static/fixed）
- `overflow-x/y: scroll` 映射为 `overflow_x_scroll`/`overflow_y_scroll`（Div 上可用）
- `aspect-ratio` 仅支持数值与 `square`，不支持 `W/H` 比式字符串
- `<anchored>`/`<deferred>` 非 Styled，CSS 须作用于内部 div

## Assumptions & Decisions

1. **范围包含 GPUI 原语 anchored/deferred**：用户评判标准"构建 gpui-component 所有组件完整能力"要求覆盖层（Popover/Tooltip/Dropdown/Modal），而 GPUI 无 z_index，必须经 deferred 实现 z-order、anchored 实现定位。用户所言"基本节点都是 gpui 原生"支持将 anchored/deferred 作为 GPUI 原语纳入 builtin。`canvas` 不纳入（无 gpui-component 组件依赖原始画布绘制，且 paint 回调不符合声明式模型）。

2. **`<a>` 不改动**：现 `<a>` → div，href 静默丢弃。链接行为需用户加 `on-click`，符合"从基础元素构建"的心智。保持现状。

3. **overflow 修复为 P0**：当前生成不存在的 `overflow_x_scrollbar` 是潜在编译错误，必须最先修复。

4. **`is_styled` 字段而非每元素自定义 translator**：anchored 复用引擎的子节点/if/show/each 处理，仅通过 `is_styled=false` 跳过 CSS 与样式属性。deferred 因构造器签名特殊（child 入参）必须自定义 translator。

5. **Grid 列为 P2**：gpui-component 多数组件用 flexbox 而非 grid，但 Table/DescriptionList 等可能用 grid，故补齐但优先级较低。

6. **不改 props_registry/tags**：builtin 标签不经 COMPONENT_PROPS 注册（已确认 builtin 不在 component_lookup 中），新增 anchored/deferred 同样作为 builtin 走 `register_all`，不走组件属性注册表。

## Verification Steps

每个 Task 完成后运行对应单测，全部完成后整体验证：

1. **Task 1 后**：`cargo test -p rust-rml-engine --lib css::mapper::tests overflow` + `cargo test -p rust-rml-engine --lib compiler::codegen::style_attr::tests overflow`
2. **Task 2 后**：`cargo test -p rust-rml-engine --lib css::mapper::tests` + `cargo test -p rust-rml-engine --lib compiler::codegen::style_attr::tests`
3. **Task 3 后**：同上（grid 相关用例）
4. **Task 4 后**：`cargo build -p rust-rml-engine`（验证 is_styled 字段 + anchored 注册编译）+ 新增 anchored 转译单测
5. **Task 5 后**：`cargo build -p rust-rml-engine`（验证 deferred 编译）+ 新增 deferred 转译单测
6. **整体**：`cargo test -p rust-rml-engine --lib`（全量单测通过）+ `cargo build --workspace`（全工作区编译通过）

**最终评判**：用一个 `.rml` + `.css` demo 验证可基于 div/anchored/deferred + 完整 CSS 表达一个 Popover 风格组件（anchored 定位 + deferred z-order + 内部 div 样式），确认基础元素完备性达成。

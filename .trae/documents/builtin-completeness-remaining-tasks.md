# Builtin 基本节点完备性 — 剩余任务（Task 4-6）

## Summary

延续上一会话，Task 1-3（overflow 方法名修正、P1 CSS 映射补齐、CSS Grid 映射）已完成并验证通过。本计划覆盖剩余三项任务：

- **Task 4**：新增 `<anchored>` 原语 + BuiltinMeta 增加 `is_styled` 字段（区分 Styled / 非 Styled 元素）
- **Task 5**：新增 `<deferred>` 原语（自定义 translator，因 Deferred 非 ParentElement）
- **Task 6**：全量编译 + 测试验证

## Current State Analysis

### 已完成（Task 1-3）

- `crates/engine/src/css/mapper.rs`：overflow-x/y 方法名修正（`overflow_x_scroll`/`overflow_y_scroll`）；新增 13 个 P1 CSS arm（text-decoration/font-style/align-self/align-content/border-x/border-y/border-style/4 角圆角/flex-grow/flex-shrink/flex-basis/aspect-ratio）；新增 8 个 CSS Grid arm
- `crates/engine/src/compiler/codegen/style_attr.rs`：`is_style_attr` 白名单同步扩展，72 项测试通过

### 待实施（Task 4-6）

- `crates/engine/src/compiler/translator/builtin/meta.rs`：`BuiltinMeta` 无 `is_styled` 字段；`translate` 签名无 `is_styled` 参数；CSS 块和属性循环无 Styled 守卫
- 21 个现有 builtin 元素文件：META 初始化无 `is_styled` 字段
- `img.rs`：translate 调用未传 `is_styled`
- `attribute.rs`：无 `anchor`/`priority`/`position`/`offset` setter
- 无 `anchored.rs` / `deferred.rs` 文件
- `mod.rs`：未注册 anchored/deferred

### GPUI API 权威参照

**anchored**（`gpui/src/elements/anchored.rs`）：
- `pub fn anchored() -> Anchored` — 无参构造
- 实现 `ParentElement`（可 `.child()`）
- **未实现 `Styled`** — CSS/样式属性须跳过
- setter：`.anchor(Anchor)` / `.position(Point<Pixels>)` / `.offset(Point<Pixels>)` / `.position_mode(AnchoredPositionMode)` / `.snap_to_window()`

**deferred**（`gpui/src/elements/deferred.rs`）：
- `pub fn deferred(child: impl IntoElement) -> Deferred` — **child 必须作构造参数**
- **未实现 `ParentElement`** — 不能 `.child()`
- **未实现 `Styled`** — CSS/样式属性须跳过
- setter：`.with_priority(usize)` / `.priority(usize)`

**Anchor 枚举**（`gpui/src/geometry.rs` L2165-2182）：
- 8 个变体：TopLeft / TopRight / BottomLeft / BottomRight / TopCenter / BottomCenter / LeftCenter / RightCenter

## Proposed Changes

### Task 4：is_styled 字段 + `<anchored>` 原语

#### 4a. `meta.rs` — BuiltinMeta 增加 is_styled 字段 + translate 签名扩展

**文件**：`crates/engine/src/compiler/translator/builtin/meta.rs`

**改动 1**：`BuiltinMeta` 结构体新增字段

```rust
pub struct BuiltinMeta {
    pub tag: &'static str,
    pub display_name: &'static str,
    pub category: ComponentCategory,
    pub ctor: &'static str,
    pub is_container: bool,
    pub is_self_closing: bool,
    /// 是否实现 GPUI `Styled` trait（anchored/deferred 等非 Styled 元素为 false）
    pub is_styled: bool,
}
```

**改动 2**：`translate` 签名新增 `is_styled: bool` 参数

```rust
pub fn translate(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
    ctor: &str,
    is_styled: bool,  // 新增
) -> Result<(String, bool), CodegenError> {
```

**改动 3**：CSS 块加 `is_styled` 守卫（L134-140）

```rust
// 2b. 应用 CSS 样式（class/id 属性匹配全局样式表）—— 非 Styled 元素跳过
if is_styled {
    if let Some(sheet) = &ctx.stylesheet {
        let style_code = apply_css_styles(elem, tag, sheet, parents);
        if !style_code.is_empty() {
            code.push_str(&style_code);
        }
    }
}
```

**改动 4**：属性循环 Static 分支跳过样式属性（L143-156）

在 `Attribute::Static { name, value, .. } => {` 分支体内，`apply_static_attr` 调用前加入守卫：

```rust
Attribute::Static { name, value, .. } => {
    // 非 Styled 元素跳过 style 内联样式 + 归一化样式属性
    if !is_styled && (name == "style" || crate::compiler::codegen::style_attr::is_style_attr(name)) {
        continue;
    }
    code.push_str(&apply_static_attr(name, value));
}
```

**改动 5**：`BuiltinTranslator::to_rust` 传递 `is_styled`（L56）

```rust
builtin_engine::translate(elem, ctx, id_counter, loop_vars, parents, self.meta.ctor, self.meta.is_styled)
```

#### 4b. 21 个现有 builtin 元素文件 — 补 `is_styled: true`

每个文件的 META 初始化在 `is_self_closing: <bool>,` 行后补 `is_styled: true,`。

文件清单（21 个）：
- `div.rs` / `span.rs` / `p.rs` / `h1.rs` / `h2.rs` / `h3.rs` / `h4.rs` / `h5.rs` / `h6.rs`
- `button.rs` / `input.rs` / `textarea.rs`（is_self_closing: false）
- `ul.rs` / `ol.rs` / `li.rs`
- `img.rs` / `svg.rs` / `a.rs` / `label.rs` / `br.rs` / `code.rs`

#### 4c. `img.rs` — translate 调用补 `true` 参数

**文件**：`crates/engine/src/compiler/translator/builtin/img.rs` L47-49

```rust
super::meta::builtin_engine::translate(
    elem, ctx, id_counter, loop_vars, parents, &ctor, true,
)
```

#### 4d. 创建 `anchored.rs`

**文件**：`crates/engine/src/compiler/translator/builtin/anchored.rs`

```rust
//! `<anchored>` translator —— 映射到 GPUI 原生 `gpui::anchored()`
//!
//! Anchored 实现 `ParentElement` 但未实现 `Styled`，因此 `is_styled: false`。
//! CSS / 归一化样式属性由 `builtin_engine::translate` 的 `is_styled` 守卫自动跳过，
//! 样式应作用于其内部子元素（如 `<div>` 包装层）。

use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

const META: &BuiltinMeta = &BuiltinMeta {
    tag: "anchored",
    display_name: "Anchored",
    category: ComponentCategory::Layout,
    ctor: "gpui::anchored()",
    is_container: true,
    is_self_closing: false,
    is_styled: false,
};

#[derive(Debug)]
pub struct AnchoredTranslator;

impl IRmlTranslator for AnchoredTranslator {
    fn tag(&self) -> &'static str {
        META.tag
    }

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

    fn to_rml(
        &self,
        elem: &Element,
        ctx: &super::PrinterCtx,
    ) -> Result<String, super::PrintError> {
        BuiltinTranslator { meta: META }.to_rml(elem, ctx)
    }

    fn metadata(&self) -> super::TranslatorMetadata {
        META.to_metadata()
    }
}
```

#### 4e. `attribute.rs` — 新增 anchor/offset/snap-to-window setter

**文件**：`crates/engine/src/compiler/codegen/attribute.rs`

在 `apply_static_attr` 的 match 中，`"path" => ...` 行之后、deprecated arm 之前，新增：

```rust
// anchored 专用：anchor 定位角（8 变体）
"anchor" => match value {
    "top-left" => ".anchor(gpui::Anchor::TopLeft)".to_string(),
    "top-right" => ".anchor(gpui::Anchor::TopRight)".to_string(),
    "bottom-left" => ".anchor(gpui::Anchor::BottomLeft)".to_string(),
    "bottom-right" => ".anchor(gpui::Anchor::BottomRight)".to_string(),
    "top-center" => ".anchor(gpui::Anchor::TopCenter)".to_string(),
    "bottom-center" => ".anchor(gpui::Anchor::BottomCenter)".to_string(),
    "left-center" => ".anchor(gpui::Anchor::LeftCenter)".to_string(),
    "right-center" => ".anchor(gpui::Anchor::RightCenter)".to_string(),
    _ => {
        eprintln!(
            "[rml warning] unknown anchor value `{}`, expected one of: \
             top-left, top-right, bottom-left, bottom-right, top-center, \
             bottom-center, left-center, right-center",
            value
        );
        String::new()
    }
},
// anchored 专用：offset 偏移量 "x,y"（如 "10px,5px"）
"offset" => parse_point_method("offset", value),
// anchored 专用：snap_to_window 布尔
"snap-to-window" => {
    if value == "true" {
        ".snap_to_window()".to_string()
    } else {
        String::new()
    }
}
```

> **注意：不暴露 `.position(Point<Pixels>)` 为 RML 属性**。`position` 已在 `is_style_attr` 白名单中（CSS `position: absolute/relative`），若为 anchored 暴露同名属性，非 Styled 守卫 `is_style_attr("position")` 会错误跳过。anchored 的定位通过 `anchor` + `offset` 组合覆盖主流 popover/tooltip 场景；绝对窗口坐标的 `.position()` 为高级用例，如需可通过 code-based 方式调用。

在文件末尾（`apply_inline_style` 之后）新增辅助函数：

```rust
/// 解析 "x,y" 坐标字符串为 `.method_name(gpui::point(gpui::px(x), gpui::px(y)))` 调用
///
/// 支持格式：`"10px,20px"` / `"10,20"` / `"10px 20px"`
fn parse_point_method(method_name: &str, value: &str) -> String {
    let parts: Vec<&str> = value.split([',', ' ']).filter(|s| !s.trim().is_empty()).collect();
    if parts.len() != 2 {
        eprintln!(
            "[rml warning] invalid point value `{}` for {}, expected \"x,y\" (e.g. \"10px,20px\")",
            value, method_name
        );
        return String::new();
    }
    let x = parse_px_value(parts[0].trim());
    let y = parse_px_value(parts[1].trim());
    match (x, y) {
        (Some(xv), Some(yv)) => format!(
            ".{}(gpui::point(gpui::px({}), gpui::px({})))",
            method_name, xv, yv
        ),
        _ => {
            eprintln!(
                "[rml warning] invalid point component in `{}` for {}",
                value, method_name
            );
            String::new()
        }
    }
}

/// 解析 "10px" / "10" / "10.5" 为 f32 值
fn parse_px_value(s: &str) -> Option<f32> {
    let s = s.trim().trim_end_matches("px").trim();
    s.parse::<f32>().ok()
}
```

#### 4f. `mod.rs` — 注册 anchored

**文件**：`crates/engine/src/compiler/translator/builtin/mod.rs`

- 在模块声明区（`pub mod br;` 之后）新增 `pub mod anchored;`
- 在 `register_all` 函数末尾新增 `registry.register(anchored::AnchoredTranslator);`

### Task 5：`<deferred>` 原语（自定义 translator）

#### 5a. 创建 `deferred.rs`

**文件**：`crates/engine/src/compiler/translator/builtin/deferred.rs`

Deferred 非 ParentElement（child 须作构造参数），不能用 BuiltinTranslator，需自定义 translator。

```rust
//! `<deferred>` translator —— 映射到 GPUI 原生 `gpui::deferred(child)`
//!
//! Deferred 延迟子元素绘制（用于 z-order 控制 / overlay 渲染）。
//! 与其他 builtin 元素不同，Deferred 非 ParentElement —— child 必须作为
//! `gpui::deferred(child)` 构造参数传入，而非 `.child()` 链式调用。
//! 因此本 translator 不复用 `BuiltinTranslator`，自行生成构造代码。
//!
//! `priority` 属性映射到 `.with_priority(N)`，控制 z-order（越大越上层）。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::node::gen_node_impl;
use crate::compiler::codegen::text::gen_expr_code;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, Node};

const TAG: &str = "deferred";
const DISPLAY_NAME: &str = "Deferred";

#[derive(Debug)]
pub struct DeferredTranslator;

impl IRmlTranslator for DeferredTranslator {
    fn tag(&self) -> &'static str {
        TAG
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        // 1. 提取 priority 属性（默认 0）
        let priority: usize = elem
            .attributes
            .iter()
            .find_map(|attr| match attr {
                Attribute::Static { name, value, .. } if name == "priority" => {
                    value.parse::<usize>().ok()
                }
                _ => None,
            })
            .unwrap_or(0);

        // 2. 生成唯一子元素代码
        if elem.children.is_empty() {
            return Err(CodegenError {
                message: "`<deferred>` 必须包含且仅包含一个子元素".to_string(),
                span: Some(elem.span),
            });
        }
        let child_node = &elem.children[0];
        let (child_code, is_iter) =
            gen_node_impl(child_node, ctx, 0, id_counter, loop_vars, parents)?;
        if is_iter {
            return Err(CodegenError {
                message: "`<deferred>` 的子元素不支持 `each` 指令".to_string(),
                span: Some(elem.span),
            });
        }

        // 3. 构造 deferred(child).with_priority(N)
        let mut code = format!("gpui::deferred({{{}}})", child_code);
        if priority > 0 {
            code.push_str(&format!(".with_priority({})", priority));
        }

        // 4. 处理 if / show 指令（不支持 each）
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

        let if_cond: Option<String> = elem.directives.iter().find_map(|d| match d {
            Directive::If { expr: c, .. } => Some(c.clone()),
            _ => None,
        });
        let show_cond: Option<String> = if if_cond.is_some() {
            None
        } else {
            elem.directives.iter().find_map(|d| match d {
                Directive::Show { expr: c, .. } => Some(c.clone()),
                _ => None,
            })
        };

        if let Some(cond) = if_cond {
            let cond_code = gen_expr_code(&cond, &lv, &computed);
            let cond_code = cond_code
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .map(|s| s.to_string())
                .unwrap_or(cond_code);
            code = format!(
                "if {} {{ {}.into_any_element() }} else {{ gpui::Empty.into_any_element() }}",
                cond_code, code
            );
        } else if let Some(cond) = show_cond {
            let cond_code = gen_expr_code(&cond, &lv, &computed);
            let cond_code = cond_code
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .map(|s| s.to_string())
                .unwrap_or(cond_code);
            code = format!("{}.when(!{}, |d| d.invisible())", code, cond_code);
        }

        Ok((code, false))
    }

    fn to_rml(
        &self,
        elem: &Element,
        ctx: &PrinterCtx,
    ) -> Result<String, super::PrintError> {
        // 复用 builtin_engine::print（deferred 标签非 void，正常容器序列化）
        super::meta::builtin_engine::print(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new(TAG, DISPLAY_NAME, ComponentCategory::Layout).container(true)
    }
}
```

#### 5b. `mod.rs` — 注册 deferred

- 在模块声明区新增 `pub mod deferred;`
- 在 `register_all` 函数末尾新增 `registry.register(deferred::DeferredTranslator);`

### Task 6：验证

#### 6a. 编译验证

```sh
cargo build -p rust-rml-engine
```

#### 6b. 测试验证

```sh
cargo test -p rust-rml-engine --lib
```

重点验证：
- `css::mapper::tests` — 全部 CSS 映射测试通过
- `compiler::codegen::style_attr::tests` — 全部样式属性测试通过
- 新增的 `is_styled` 字段不破坏现有 21 个 builtin 元素的 META 初始化

#### 6c. 完整工作区编译

```sh
cargo build --workspace
```

## Assumptions & Decisions

1. **is_styled 守卫位置**：在 `translate` 的属性循环中守卫，而非在 `apply_static_attr` 内部。原因：`is_styled` 上下文仅在 translate 时可知，且 `apply_static_attr` 可能被其他（Styled）路径复用。

2. **anchored 属性集**：仅暴露 `anchor`（8 变体）/ `offset`（"x,y"）/ `snap-to-window`（bool）三个属性。**不暴露 `.position()`**，因 `position` 已在 `is_style_attr` 白名单中（CSS `position: absolute/relative`），非 Styled 守卫会错误跳过。`anchor` + `offset` 覆盖主流 popover/tooltip 定位场景。`position_mode` 的 `AnchoredPositionMode` 枚举可见性未确认，暂不暴露。

3. **deferred 不支持 each 指令**：语义上 deferred 是单子元素 z-order 包装器，each 迭代应在外层 div 进行。

4. **deferred 不支持 ref/key/events**：deferred 是布局原语，非交互元素；如需交互，在其子元素上添加。

5. **deferred 的 to_rml 复用 builtin_engine::print**：序列化逻辑与普通容器标签一致（非 void，有开闭标签）。

6. **parse_point_method 辅助函数**：仅用于 anchored 的 `offset` 属性，解析 `"10px,20px"` 格式。

7. **属性名命名约定**：`anchor`/`offset`/`snap-to-window` 均不在 `is_style_attr` 白名单中，与非 Styled 守卫无冲突。`snap-to-window` 用 kebab-case（与 CSS 多词属性约定一致）。

## Verification Steps

1. **meta.rs 编译通过**：`is_styled` 字段添加后，所有 META 初始化必须同步更新（编译器强制）
2. **21 个 builtin 文件全部补 `is_styled: true`**：编译器逐个报错引导，确保无遗漏
3. **img.rs 调用签名匹配**：`translate(..., &ctor, true)` 七参数调用
4. **anchored 生成代码验证**：`<anchored anchor="top-right"><div>...</div></anchored>` 生成 `gpui::anchored().anchor(gpui::Anchor::TopRight).child(...)`
5. **deferred 生成代码验证**：`<deferred priority="1"><div>...</div></deferred>` 生成 `gpui::deferred({gpui::div()...}).with_priority(1)`
6. **非 Styled 元素 CSS 跳过验证**：`<anchored class="foo">` 不生成 `.foo` 对应的样式方法链
7. **全量测试通过**：`cargo test -p rust-rml-engine --lib` 全绿
8. **全量编译通过**：`cargo build --workspace` 无错误

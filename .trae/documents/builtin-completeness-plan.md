# Builtin 基础元素能力完备性补齐计划

## Context

RML 框架的 builtin 基础元素（`crates/engine/src/compiler/translator/builtin/`，对标 HTML 原生标签）当前能力严重不达标，无法满足评判标准："可以基于 `.rml` + `.css` 开发高级组件，可以基于这些基础元素构建 gpui-component 所有组件完整能力"。

**三大缺失**：
1. **元素映射缺失**：builtin 全部是 `gpui::div()` 别名，未映射到 GPUI 真正的原生元素（`img()`/`svg()`/`anchored()`/`deferred()`/`canvas()`）。`<img src>` 的 src 被丢弃不渲染图片，无 `<svg>` 标签。
2. **样式映射缺口大**：`mapper.rs` 只覆盖盒模型/文本基础/Flexbox/overflow/opacity。定位（position/top/right/bottom/left）、box-shadow、cursor、visibility、align-self/align-content、text-decoration、text-overflow/ellipsis/truncate、CSS Grid、aspect-ratio、italic 等均未映射——而这些是 gpui-component 组件普遍使用的能力（80 处调用验证）。
3. **HTML 语义属性缺失**：`<img src>`/`<a href>`/`<input type/value/placeholder>`/`<button type/disabled>` 等语义属性被 [attribute.rs#L25](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L25) 直接丢弃。

**架构事实**（影响设计）：
- GPUI `Styled` trait 无 `z_index` 方法，z-order 通过 `deferred` 元素实现
- `mapper.rs` L139/L144 生成的 `overflow_x/y_scrollbar()` 依赖 gpui-component 的 `ScrollableElement` trait（非 GPUI 原生），是隐式依赖 smell
- 样式映射已是单一信源（[mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) 的 `map_declaration`），架构健康，补齐集中

## 现状诊断

### GPUI 原生元素清单（[elements/mod.rs](file:///c:/Users/lusid/.cargo/git/checkouts/zed-23861290b5d2093f/1d217ee/crates/gpui/src/elements/mod.rs)）
`div`/`svg`/`img`/`text`/`canvas`/`anchored`/`deferred`/`list`/`uniform_list`/`animation`/`surface`/`image_cache`

### RML builtin 现状
- 已有 15 个标签：div/span/p/h1-h6/button/input/textarea/ul/ol/li/img/a/label/br/code
- 除 input/textarea 有 model 绑定外，全部 `ctor: "gpui::div()"`
- 缺 `<svg>`/`<canvas>`/`<anchored>`/`<deferred>` 标签

### GPUI Styled 能力 vs RML 映射对照
**已映射**：盒模型（padding/margin/width/height/min-*/max-*/border/border-color/border-radius/border-t/b/r/l）、文本（font-size/weight/family/text-align/line-height/white-space/color/background）、Flexbox 基础（display flex/none/flex-direction/flex-wrap/justify-content/align-items/flex/gap）、视觉（opacity/overflow/overflow-x/overflow-y）

**未映射（GPUI 支持）**：见下方补齐方案

## 补齐方案

### 一、样式映射补齐（mapper.rs + style_attr.rs）

在 `map_declaration` 补 match 分支，在 `is_style_attr` 补 kebab-case 判定。每项补单元测试。

| 类别 | CSS 属性 | GPUI 方法 | 优先级 |
|---|---|---|---|
| 定位 | `position`/`top`/`right`/`bottom`/`left`/`inset` | `.absolute()`/`.relative()`/`.top(v)`/`.right(v)`/`.bottom(v)`/`.left(v)`/`.inset(v)` | P0 |
| 阴影 | `box-shadow`（快捷词 `2xs/xs/sm/md/lg/xl/2xl/none`） | `.shadow_sm()` 等 | P0 |
| 交互 | `cursor`（`pointer/default/text/move/not-allowed/...`） | `.cursor_pointer()` 等 | P0 |
| 可见性 | `visibility`（`visible/hidden`） | `.visible()`/`.invisible()` | P0 |
| 文本截断 | `text-overflow: ellipsis`、`line-clamp: <n>`、`truncate` | `.text_ellipsis()`/`.line_clamp(n)`/`.truncate()` | P0 |
| 文本装饰 | `text-decoration`（`underline/line-through/none`） | `.underline()`/`.line_through()`/`.text_decoration_none()` | P1 |
| 字体风格 | `font-style: italic/normal` | `.italic()`/`.not_italic()` | P1 |
| flex 补全 | `flex-direction: column-reverse/row-reverse`、`flex-wrap: wrap-reverse`、`justify-content: space-around/space-evenly`、`align-items: baseline` | 对应方法 | P1 |
| align-self | `align-self`（`start/end/center/stretch/baseline`） | `.self_start()` 等 | P1 |
| align-content | `align-content`（`center/start/end/between/around/evenly/stretch`） | `.content_center()` 等 | P1 |
| border 细化 | `border-x`/`border-y`、`border-style: dashed` | `.border_x(v)`/`.border_y(v)`/`.border_dashed()` | P1 |
| 圆角细化 | `border-top-left-radius` 等 4 角、`border-top-radius` 等 4 边 | `.rounded_tl(v)` 等 / `.rounded_t(v)` 等 | P1 |
| 显示补全 | `display: block/grid` | `.block()`/`.grid()` | P2 |
| 比例 | `aspect-ratio: <ratio>`、`aspect-ratio: square` | `.aspect_ratio(r)`/`.aspect_square()` | P2 |
| CSS Grid | `grid-template-columns/rows`、`grid-column/row: start/end/span` | `.grid_cols(n)`/`.grid_rows(n)`/`.col_start(n)`/`.col_span(n)` 等 | P2 |

### 二、builtin 元素映射补齐

| 元素 | 现状 | 补齐方案 | 优先级 |
|---|---|---|---|
| `<img>` | div 别名，src 丢弃 | ctor 改 `gpui::img()`；处理 `src`（→图片源）、`alt`（日志） | P0 |
| `<svg>` | 不存在 | 新建 `builtin/svg.rs`；ctor `gpui::svg()`；处理 `path`（→SVG path）、`color`、`size` | P0 |
| `<a>` | div 别名，href 丢弃 | 处理 `href`（生成 on_click 打开 URL 或仅保留属性） | P1 |
| `<input>` | div 或 model 绑定 | 补 `type`/`placeholder`/`value`(static) 属性处理 | P1 |
| `<button>` | div 别名 | 默认 `cursor: pointer`；处理 `type`/`disabled` | P1 |
| `<canvas>` | 不存在 | 新建 `builtin/canvas.rs`；ctor `gpui::canvas()` | P2 |
| `<anchored>` | 不存在 | 新建 `builtin/anchored.rs`；ctor `gpui::anchored()`（浮层定位基础） | P1 |
| `<deferred>` | 不存在 | 新建 `builtin/deferred.rs`；ctor `gpui::deferred()`（z-order 基础） | P1 |

### 三、架构修复

**A. overflow 隐式依赖**
- 现状：`overflow-x/y: scroll/auto` 生成 `overflow_x/y_scrollbar()`，依赖 gpui-component `ScrollableElement`
- 方案：改用 GPUI 原生 `overflow_x/y_scroll()`（[div.rs#L1318](file:///c:/Users/lusid/.cargo/git/checkouts/zed-23861290b5d2093f/1d217ee/crates/gpui/src/elements/div.rs#L1318)），去掉隐式依赖
- Tradeoff：丢失自动可见滚动条；可见滚动条作为独立 RML 组件后续迭代
- 影响文件：[mapper.rs#L137-L146](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs#L137-L146)、对应测试

**B. z-index 文档化**
- GPUI 无 `z_index`，CSS `z-index` 属性在 RML 里：报 warning 并丢弃，文档化"用 `<deferred>` 元素实现 z-order"

## 实施顺序

1. **P0 样式映射**（定位/阴影/cursor/visibility/text-overflow/truncate）—— 不引入新依赖，立即解锁浮层/交互/文本截断
2. **P0 元素映射**（img 真实化、svg 新增）—— 解锁图片和图标
3. **P1 样式映射**（文本装饰/font-style/flex 补全/align-self/content/border 细化/圆角细化）
4. **P1 元素映射**（a/input/button 语义补齐、anchored/deferred 新增）
5. **架构修复**（overflow 依赖、z-index 文档化）
6. **P2 样式映射**（display block/grid、aspect-ratio、CSS Grid）
7. **P2 元素映射**（canvas）
8. **测试补齐**（贯穿每步，mapper 测试 + props_registry 一致性测试）

## 关键文件

**样式映射**（单一信源）：
- [crates/engine/src/css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) — `map_declaration` 补 match 分支
- [crates/engine/src/compiler/codegen/style_attr.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs) — `is_style_attr` 补判定

**元素映射**：
- [crates/engine/src/compiler/translator/builtin/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/) — 各标签 translator
- [crates/engine/src/compiler/translator/builtin/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/mod.rs) — `register_all` 注册新标签
- [crates/engine/src/compiler/translator/builtin/meta.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/meta.rs) — `BuiltinTranslator` 引擎（已有，无需改）

**属性处理**：
- [crates/engine/src/compiler/codegen/attribute.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs) — `apply_static_attr` 补 src/href/type/placeholder 等
- [crates/engine/src/compiler/codegen/binding.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/binding.rs) — input model 绑定

**GPUI 能力参照**（只读）：
- [gpui styled.rs](file:///c:/Users/lusid/.cargo/git/checkouts/zed-23861290b5d2093f/1d217ee/crates/gpui/src/styled.rs) — Styled trait 方法
- [gpui_macros styles.rs](file:///c:/Users/lusid/.cargo/git/checkouts/zed-23861290b5d2093f/1d217ee/crates/gpui_macros/src/styles.rs) — 宏生成方法
- [gpui elements/](file:///c:/Users/lusid/.cargo/git/checkouts/zed-23861290b5d2093f/1d217ee/crates/gpui/src/elements/) — 原生元素

## 验证方式

1. **单元测试**：`cargo test -p rust-rml-engine` — mapper.rs / style_attr.rs 每个新 match 分支补测试
2. **属性注册一致性**：`cargo test -p rust-rml-engine --lib props_registry::tests`
3. **集成验证**：用 `.rml` + `.css` 复现 gpui-component 代表性组件片段（Button 卡片样式、Tooltip 绝对定位、Icon SVG、列表 truncate），确认生成代码可编译
4. **编译验证**：`cargo build -p rust-rml-engine` 确认生成代码引用的 GPUI 方法真实存在

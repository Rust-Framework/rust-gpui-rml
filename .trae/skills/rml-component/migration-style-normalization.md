# 样式归一化迁移指南

## 概述

RML 将散落的 Tailwind 式样式属性统一归一化为 CSS 子集命名的一等直接属性。本指南描述：

- 旧 Tailwind 式属性 → 归一化属性的完整映射表
- deprecation warning 触发条件
- bind 形式不支持的 warning
- CodeEditor `h-full` 迁移说明
- 综合迁移示例（旧 vs 新）
- 引擎代码生成路径清单（通用 vs 特例分离）

## 旧 Tailwind 式属性 → 归一化属性映射表

| 旧属性 | 新属性（等价替换） | 生成代码 |
|--------|-----------------|----------|
| `v-flex=""` | `display="flex" flex-direction="column"` | `.flex().flex_col()` |
| `h-flex=""` | `display="flex" flex-direction="row"` | `.flex().flex_row()` |
| `gap-2=""` | `gap="8px"`（N×4px） | `.gap(gpui::px(8.0))` |
| `gap-4=""` | `gap="16px"` | `.gap(gpui::px(16.0))` |
| `gap-6=""` | `gap="24px"` | `.gap(gpui::px(24.0))` |
| `h-full=""` | `height="full"` | `.h_full()` |
| `w-full=""` | `width="full"` | `.w_full()` |
| `min-w-0=""` | `min-width="0"` | `.min_w_0()` |
| `min-h-0=""` | `min-height="0"` | `.min_h_0()` |
| `items-center=""` | `align-items="center"` | `.items_center()` |
| `flex-wrap=""` | `flex-wrap="wrap"` | `.flex_wrap()` |
| `p-N=""` | `padding="N*4px"` | `.p(gpui::px(N*4.0))` |

## deprecation warning 触发条件

在 [attribute.rs:27-36](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L27-L36) 中，以下 6 个属性被检测到时输出 `[rml deprecation]` warning 并丢弃：

- `h_flex` / `v_flex`（布局快捷方法）
- `h_full` / `w_full`（全宽全高）
- `min_w_0` / `min_h_0`（最小尺寸归零）

warning 形如：

```
[rml deprecation] `h_flex` is deprecated; use normalized CSS attribute instead (e.g. display="flex" flex-direction="row" for h-flex, width="full" for w-full, min-width="0" for min-w-0)
```

## bind 形式不支持 warning

在 [attribute.rs:137-145](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L137-L145) 中，归一化样式属性的 bind 形式（如 `width={computed}`）输出 `[rml warning]` 并丢弃：

```
[rml warning] bind form `width={computed}` is not supported for style attribute; use static form `width="..."` instead. Property will be dropped.
```

运行时动态样式应走 `class=` + 主题切换路径。

## CodeEditor `h-full` 迁移说明

### 旧写法（已废弃）

```xml
<CodeEditor h-full="" value={code_sample} language="rml" />
```

### 新写法

```xml
<CodeEditor height="full" value={code_sample} language="rml" />
```

### 实现原理

[gen.rs:137-140](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs#L137-L140) CodeEditor 通过 `if !has("height") && !has("h")` 守卫应用默认 `.h(gpui::px(360.))`。用户写 `height="full"` 时，`has("height")` 返回 true 跳过默认调用，仅由 `component_static_setter` → `apply_style_attr` 生成 `.h_full()` 追加到 setter 链，避免重复设置。

## 综合迁移示例

### 旧（多 Tailwind 式属性）

```xml
<div v-flex="" gap-2="" h-full="">
    <div h-flex="" gap-4="" items-center="">
        <Avatar name="Alice" />
        <p>用户名</p>
    </div>
    <CodeEditor h-full="" />
</div>
```

### 新（归一化 CSS 属性）

```xml
<div display="flex" flex-direction="column" gap="8px" height="full">
    <div display="flex" flex-direction="row" gap="16px" align-items="center">
        <Avatar name="Alice" />
        <p>用户名</p>
    </div>
    <CodeEditor height="full" />
</div>
```

## 引擎代码生成路径清单

### 通用路径（高内聚，单一映射源）

| 路径 | 入口 | 处理范围 |
|------|------|---------|
| **A. 原生元素通用样式** | `apply_static_attr` → `style_attr::apply_style_attr` | `<div>`/`<span>` 等 |
| **B. 扩展组件通用样式** | `component_static_setter` → `style_attr::apply_style_attr` | `<Button>`/`<Card>` 等 |

两条路径共用同一入口 `style_attr::apply_style_attr` 与同一映射源 `css::map_declarations`，避免双轨制。

### 特例路径（低耦合，明确隔离）

| 路径 | 入口 | 处理范围 | 行为 |
|------|------|---------|------|
| **C. deprecation** | `apply_static_attr` match 臂 | `h_flex`/`v_flex`/`h_full`/`w_full`/`min_w_0`/`min_h_0` | warning + 丢弃 |
| **D. bind 不支持** | `apply_bind_attr` _ 分支 | `is_style_attr(name)` 为真 | warning + 丢弃 |
| **E. CodeEditor 默认高度** | `gen.rs height_chain` | CodeEditor 通过 `has("height")` 守卫应用默认 `.h(360px)`，用户写 `height="full"` 时跳过默认仅生成 `.h_full()` | has 守卫跳过默认 |

### 单一映射源验证

所有"通用样式"路径（A/B/E）均复用 [css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) `map_declarations`，无独立映射逻辑。

## 不在归一化范围

- **字体权重快捷方法保留**：`font_bold` / `font_semibold` 等 `StyledExt` 方法保留为快捷写法，不强制迁移到 `font-weight="bold"`
- **bind 形式样式属性**：当前不支持 `width={computed}` 形式（运行时动态样式走 `class=` + 主题切换）
- **新增 CSS 属性**：`box-shadow` / `transform` 等 `mapper.rs` 未支持的属性不在归一化范围
- **事件归一化**：事件已通过 `on-*` 命名归一化（详见 [03-property-classification.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md)）

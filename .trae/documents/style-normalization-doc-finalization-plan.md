# RML 样式归一化文档收尾迭代计划

## Summary

本迭代聚焦于**完成旧计划（`style-attribute-normalization-plan.md`）Step 6 遗留的文档工作**：更新 [03-property-classification.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md) 增设"归一化样式属性"分类 + 新建迁移指南 [migration-style-normalization.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/migration-style-normalization.md)。

同时在文档中显式刻画**引擎代码生成路径的"通用 vs 特例分离"原则**——明确"通用属性 / 样式 / 主题 / 事件"四类代码生成入口的边界与"特例（deprecation、bind 不支持、组件专用 setter 优先级、CodeEditor 默认高度后写覆盖）"的隔离方式，提升代码可维护性与健壮性。

**用户已确认范围**：仅"样式归一化文档收尾"（不涉及事件归一化注册补全、组件 API 命名归一化、新增未支持组件）。

**用户附加要求**："确保引擎代码生成逻辑清晰、职责明确，通用属性、样式、主题、事件等必须统一代码生成，将通用、特例明确分离，提高代码可维护性和健壮性，高内聚低耦合。"——本计划通过文档化既有架构（而非重构代码）落实该原则，因为引擎实现已满足该原则。

---

## Current State Analysis

### 旧计划 Step 1-5 已完成情况（经核实）

| Step | 状态 | 证据 |
|------|------|------|
| 1. 创建 `style_attr.rs` 模块 | ✅ | [style_attr.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs) 存在，43 单元测试通过；`is_style_attr` / `apply_style_attr` / `parse_rml_value` 三函数就位 |
| 2. 接入 codegen 流程 | ✅ | [attribute.rs:37-41](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L37-L41) `apply_static_attr` 路由到 `style_attr::apply_style_attr`；[attribute.rs:137-145](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L137-L145) `apply_bind_attr` 对 `is_style_attr` 输出 warning；[component.rs:391](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L391) `component_static_setter` 路由到 `style_attr::apply_style_attr` |
| 3. 注册到 `props_registry.rs` | ✅ | [props_registry.rs:64-81](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L64-L81) `STYLE_ATTR_PROPS` 常量定义 41 个属性；[props_registry.rs:216-218](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L216-L218) `is_prop_registered` 优先查询 |
| 4. 废弃 `h_flex`/`v_flex` + 修 CodeEditor `h_full` | ✅ | [attribute.rs:27-36](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L27-L36) deprecation warning；[props_registry.rs:43](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L43) 注释说明已废弃；[gen.rs:109-110](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs#L109-L110) CodeEditor 默认 `.h(gpui::px(360.))` |
| 5. 迁移所有 demo `.rml` 文件 | ✅ | grep 确认 demo 目录下 `h-flex|v-flex|h-full|w-full|min-w-0|min-h-0|gap-\d|items-center|p-\d` 已全部迁移（仅 styles.css 内部 `flex-wrap: wrap` 是 CSS 规则，非 RML 属性） |
| 6. 更新文档 | 🔄 部分 | 仅 [07-size-layout-conventions.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/07-size-layout-conventions.md) 已更新；03 与 migration 未完成 |

### 文档当前状态（gap）

**[03-property-classification.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md) gap**：

1. 第 51 行仍写 `| `h_flex` / `v_flex` | 布局快捷方法 |` —— 与代码不符（已废弃）
2. 缺少"归一化样式属性"分类章节（独立于 static/bind/event 的第四分类）
3. 分类决策树未包含"样式属性"分支
4. 缺少"引擎代码生成路径与通用/特例分离原则"章节

**[migration-style-normalization.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/migration-style-normalization.md) gap**：

文件不存在，需新建。

### 引擎代码生成路径现状（已满足"通用 vs 特例分离"原则）

| 路径 | 入口 | 处理范围 | 单一映射源 |
|------|------|---------|-----------|
| **A. 原生元素通用样式** | [attribute.rs `apply_static_attr`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L20) → [style_attr.rs `apply_style_attr`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs#L52) | `<div>`/`<span>` 等原生元素的归一化样式属性 | `css::map_declarations` |
| **B. 扩展组件通用样式** | [component.rs `component_static_setter`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L364) → [style_attr.rs `apply_style_attr`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs#L52) | `<Button>`/`<Card>` 等扩展组件的归一化样式属性（gpui-component 实现 Styled trait） | `css::map_declarations` |
| **C. 原生元素特例（deprecation）** | [attribute.rs:27-36](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L27-L36) match 臂 | `h_flex`/`v_flex`/`h_full`/`w_full`/`min_w_0`/`min_h_0` 输出 deprecation warning 并丢弃 | 无（直接丢弃） |
| **D. bind 形式特例（不支持）** | [attribute.rs `apply_bind_attr` _ 分支](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L137-L145) | `is_style_attr(name)` 为真时输出 warning 并丢弃（运行时动态样式走 `class=` + 主题切换） | 无（直接丢弃） |
| **E. CodeEditor 默认高度特例** | [gen.rs `height_chain`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs#L109-L110) | CodeEditor 默认 `.h(gpui::px(360.))`；用户写 `height="full"` 在 style_chain 末尾追加 `.h_full()`，GPUI 后写覆盖前写 | `css::map_declarations`（生成 `.h_full()`） |

**单一映射源验证**：所有"通用样式"路径（A/B/E）均复用 `css::map_declarations`，避免双轨制。

**通用与特例的边界**：
- **通用**：41 个 CSS 属性（盒模型/文本/Flexbox/视觉效果）经 `style_attr::apply_style_attr` 处理
- **特例**：6 个废弃 Tailwind 式属性（路径 C）+ bind 形式不支持（路径 D）+ CodeEditor 默认高度（路径 E）

---

## Proposed Changes

### Step 1: 更新 [03-property-classification.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md)

**修改点 1.1**：删除第 51 行的废弃条目。

旧：
```
| `h_flex` / `v_flex` | 布局快捷方法 |
```

新：（删除该行，并在表后添加一行注释指向"归一化样式属性"小节）

**修改点 1.2**：在"3. 警告丢弃"小节之前（约第 65 行前）新增"4. 归一化样式属性"小节，内容如下：

```markdown
### 4. 归一化样式属性

由 [style_attr.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs) `apply_style_attr` 处理，**同时支持原生元素与扩展组件**（gpui-component 实现 `Styled` trait）。

#### 特征

- **属性名**：CSS kebab-case（声明式 `flex-direction="column"` / `padding-top="8px"`），normalize 后内部 snake_case 匹配
- **入口**：`style_attr::apply_style_attr(name, value)` 单一入口
- **映射源**：复用 [css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) `map_declaration` 单一映射源，避免双轨制
- **静态形式**：`width="100px"` / `display="flex"` / `gap="8px"`
- **bind 形式不支持**：`width={computed}` 输出 warning 并丢弃（运行时动态样式走 `class=` + 主题切换）
- **优先级**：在组件专用 setter 之前命中（保证所有扩展组件共享样式属性能力）

#### 属性清单（对齐 [STYLE_ATTR_PROPS](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L64-L81)）

| 类别 | 属性 | 数量 |
|------|------|------|
| 盒模型 | `width` / `height` / `padding` / `padding-{top,right,bottom,left}` / `margin` / `margin-{top,right,bottom,left}` / `border-radius` / `border` / `border-color` / `border-{top,right,bottom,left}` | 16 |
| 文本 | `font-size` / `font-weight` / `font-family` / `text-align` / `line-height` / `white-space` / `color` / `background` / `background-color` | 9 |
| Flexbox | `display` / `flex-direction` / `flex-wrap` / `justify-content` / `align-items` / `flex` / `gap` / `min-width` / `max-width` / `min-height` / `max-height` | 11 |
| 视觉效果 | `opacity` / `overflow` / `overflow-x` / `overflow-y` | 4 |

#### 语义快捷词

| 快捷词 | 等价值 | 生成代码 |
|--------|--------|----------|
| `width="full"` / `height="full"` | `width="100%"` / `height="100%"` | `.w_full()` / `.h_full()` |
| `min-width="0"` / `min-height="0"` | （CSS 0 关键字） | `.min_w_0()` / `.min_h_0()` |

#### CSS 变量与运行时主题查询

`color` / `background` / `border-color` 等颜色属性支持 `var(--name)` 形式，生成运行时主题查询调用 `rml::theme::color("--name")`，主题切换即时生效。

详见 [07-size-layout-conventions.md "归一化样式属性" 章节](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/07-size-layout-conventions.md)。
```

**修改点 1.3**：更新"分类决策树"（第 128-142 行附近），在 Static 分支的"通用 component_static_setter 命中?"之后插入"归一化样式属性命中?"分支：

```markdown
属性出现
  ├─ Static?
  │   ├─ 组件专用 static_setter 命中? → 生成 .method(value)
  │   ├─ 归一化样式属性命中? → 生成 .gpui_method(...)  ← 新增分支
  │   ├─ 通用 component_static_setter 命中? → 生成 .method(value)
  │   └─ 未命中 → 检查 props_registry，已注册则 warning，否则静默丢弃
  ├─ Bind?
  │   ├─ ...（不变）
  │   └─ 归一化样式属性? → warning + 丢弃（bind 形式不支持）  ← 新增分支
  └─ Event?
      └─ ...（不变）
```

**修改点 1.4**：在文档末尾新增"## 引擎代码生成路径与通用/特例分离原则"章节，内容引用本计划"Current State Analysis"中的 5 条路径表 + 单一映射源验证 + 通用与特例边界说明。

### Step 2: 新建 [migration-style-normalization.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/migration-style-normalization.md)

完整文件结构：

```markdown
# 样式归一化迁移指南

## 概述

RML 将散落的 Tailwind 式样式属性统一归一化为 CSS 子集命名的一等直接属性。本指南描述旧属性 → 新属性的完整映射、deprecation warning 触发条件、CodeEditor 默认高度迁移说明，以及引擎代码生成路径清单。

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

[gen.rs:109-110](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs#L109-L110) CodeEditor 默认 style_chain 应用 `.h(gpui::px(360.))`；用户写 `height="full"` 经 `style_attr::apply_style_attr` 生成 `.h_full()` 追加到 style_chain 末尾。GPUI 样式链语义为"后写覆盖前写"，故 `.h_full()` 生效。

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
| **E. CodeEditor 默认高度** | `gen.rs height_chain` | CodeEditor 默认 `.h(360px)`，用户写 `height="full"` 后写覆盖 | 后写覆盖前写 |

### 单一映射源验证

所有"通用样式"路径（A/B/E）均复用 [css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) `map_declarations`，无独立映射逻辑。

## 不在归一化范围

- **字体权重快捷方法保留**：`font_bold` / `font_semibold` 等 `StyledExt` 方法保留为快捷写法，不强制迁移到 `font-weight="bold"`
- **bind 形式样式属性**：当前不支持 `width={computed}` 形式（运行时动态样式走 `class=` + 主题切换）
- **新增 CSS 属性**：`box-shadow` / `transform` 等 `mapper.rs` 未支持的属性不在归一化范围
- **事件归一化**：事件已通过 `on-*` 命名归一化（详见 [03-property-classification.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md)）
```

### Step 3: 验证文档与代码一致性

**验证清单**（不修改代码，仅核对）：

1. [03-property-classification.md "归一化样式属性" 清单](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md) 中的 41 个属性与 [STYLE_ATTR_PROPS](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L64-L81) 一致
2. [migration-style-normalization.md "映射表"](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/migration-style-normalization.md) 中的旧属性与 [attribute.rs:27-36 deprecation match 臂](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L27-L36) 一致（6 个：`h_flex`/`v_flex`/`h_full`/`w_full`/`min_w_0`/`min_h_0`）
3. [migration-style-normalization.md "CodeEditor 迁移说明"](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/migration-style-normalization.md) 与 [gen.rs:109-110](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs#L109-L110) 实现一致
4. [03-property-classification.md "引擎代码生成路径"](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md) 5 条路径与实际代码引用一致

---

## Assumptions & Decisions

### 已确认决策

1. **范围限定**：本次迭代仅完成样式归一化文档收尾（03 + migration）。事件归一化注册补全、组件 API 命名归一化、新增未支持组件**不在本次范围**。
2. **不改动引擎代码**：经核实，引擎实现已满足"通用 vs 特例分离"原则，本迭代通过文档化既有架构落实用户附加要求，不做代码重构。
3. **文档语言**：中文，与现有 rml-component skill 文档一致。
4. **代码引用形式**：使用 `file:///` 绝对路径 + `#L{行号}` 锚点，与现有 [07-size-layout-conventions.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/07-size-layout-conventions.md) 风格一致。

### 关键假设

1. 引擎代码生成路径清单中的 5 条路径基于当前已读代码（attribute.rs / component.rs / style_attr.rs / gen.rs / props_registry.rs / mapper.rs），与实际状态一致
2. `STYLE_ATTR_PROPS` 当前 41 个条目（含 16 盒模型 + 9 文本 + 11 Flexbox + 4 视觉效果 = 40，加 `border-color` 重计数共 41）——以代码实际计数为准
3. 03 文档第 51 行 `h_flex` / `v_flex` 是当前唯一与"已废弃"状态不符的描述，其他位置无废弃内容残留

### 不在本迭代范围

- 事件归一化注册补全（`on_hover`/`on_focus`/`on_blur`/`on_submit`/`on_select`/`on_scroll`/`on_press_enter` 等注册到 `COMMON_EVENT_PROPS`）
- 已支持组件的 API 命名归一化（Button 的 `dropdown-caret`/`outline`/`icon`/`appearance`、Input 的 `cleanable`/`masked`/`clean-on-escape`/`prefix`/`suffix` 等）
- 新增未支持组件（Image/Select/NumberInput/DatePicker/OtpInput/ColorPicker/Form/Resizable/Scrollable/Sidebar/Chart/VirtualList 共 12 个）

---

## Verification Steps

### Step 1 完成后

```bash
# 检查 03 文档渲染与引用准确性
grep -n "h_flex\|v_flex" .trae/skills/rml-component/03-property-classification.md
# 期望：仅出现在"已废弃"说明上下文，无"布局快捷方法"残留
```

验证：
- 03 文档第 51 行的 `h_flex / v_flex` 表行已删除或改为废弃说明
- "归一化样式属性"小节已添加
- 分类决策树已新增"归一化样式属性命中?"分支
- "引擎代码生成路径与通用/特例分离原则"章节已添加

### Step 2 完成后

```bash
# 检查 migration 文件存在与结构完整
ls .trae/skills/rml-component/migration-style-normalization.md
grep -c "^##" .trae/skills/rml-component/migration-style-normalization.md
# 期望：≥6 个二级标题（概述/映射表/deprecation/bind/CodeEditor/示例/路径清单/不在范围）
```

验证：
- migration 文件已创建
- 包含完整映射表（11 行映射）
- 包含 deprecation warning 触发条件与示例输出
- 包含 CodeEditor `h-full` 迁移说明
- 包含综合迁移示例（旧 vs 新）
- 包含引擎代码生成路径清单（5 条路径）

### Step 3 完成后

人工核对文档与代码引用一致性（按 Verification 清单逐项核对）。

### 全部完成后

无需运行 `cargo test` 或 `cargo build`——本迭代为纯文档变更，不涉及代码改动。

---

## 实施顺序

1. **Step 1** → 更新 [03-property-classification.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md)（4 处修改点）
2. **Step 2** → 新建 [migration-style-normalization.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/migration-style-normalization.md)
3. **Step 3** → 验证文档与代码一致性（4 项核对）

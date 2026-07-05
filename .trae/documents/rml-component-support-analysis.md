# RML 框架组件支持分析

## 概述

本文档基于 RML 框架代码库的全面调研，系统性整理 RML 框架在组件支持方面的现状，包括：

1. **通用属性配置** — CSS/CSSStyles 定制化能力、覆盖率与差距
2. **组件特定属性** — 各组件专属属性及声明式覆盖情况
3. **已支持组件清单** — 声明式覆盖、完成度、质量评估

---

## 一、通用属性配置（CSS / CSSStyles）

### 1.1 架构机制

RML 的样式系统通过三个层级实现：

| 层级 | 机制 | 说明 |
|------|------|------|
| **CSS 文件** | `.css` 样式表 | 通过 `build.rs` 的 `with_style("styles.css")` 注册，编译期解析为 `StyleSheet` AST |
| **内联样式** | `style="prop: val;"` | 在元素上直接使用 `style` 属性，通过 CSS 解析器映射 |
| **绑定样式** | `style={expr}` | ⚠️ **不支持**（编译期 warning 提示） |

### 1.2 CSS 解析器能力

**文件：** `crates/engine/src/css/parser.rs`、`ast.rs`

#### 已支持的选择器

| 选择器类型 | 示例 | 状态 |
|-----------|------|------|
| 标签选择器 | `div { ... }` | ✅ 完整 |
| 类选择器 | `.card { ... }` | ✅ 完整 |
| ID 选择器 | `#main { ... }` | ✅ 完整 |
| 通用选择器 | `* { ... }` | ✅ 完整 |
| 后代选择器 | `.container .title { ... }` | ✅ 完整 |
| 子选择器 | `.list > .item { ... }` | ✅ 完整 |
| 交集选择器 | `.button.primary { ... }` | ✅ 完整 |
| 分组选择器 | `h1, h2, h3 { ... }` | ✅ 完整 |
| `:root` 变量定义 | `:root { --primary: #007bff; }` | ✅ 完整 |
| 伪类选择器 | `:hover`, `:focus`, `:first-child` | ❌ **不支持** |
| 伪元素选择器 | `::before`, `::after` | ❌ **不支持** |
| 属性选择器 | `[type="text"]` | ❌ **不支持** |
| 相邻/兄弟选择器 | `+`, `~` | ❌ **不支持** |

#### 已支持的属性值类型

| 值类型 | 示例 | 状态 |
|--------|------|------|
| 长度（px/pt） | `10px`, `12pt` | ✅ 完整 |
| 长度（em/rem） | `1.5em`, `2rem` | ✅ 解析但映射为 px（等比例转换） |
| 百分比 | `50%` | ✅ 解析但不支持映射（长度方法仅处理 px） |
| vw/vh | `100vw`, `100vh` | ✅ 解析但不支持映射 |
| 十六进制颜色 | `#ff0000`, `#f00`, `#ff0000ff` | ✅ 完整 |
| 命名颜色 | `red`, `blue`, `transparent` 等 | ✅ 11 种基础颜色 |
| CSS 变量 | `var(--primary)`, `var(--spacing, 8px)` | ✅ 完整（颜色变量运行时查询，非颜色构建期内联） |
| 函数值 | `rgba(0,0,0,0.5)` | ✅ 解析但不映射（直接生成 GPUI color） |
| 字符串字面量 | `'Arial'` | ✅ 解析但无用 |
| 简写值列表 | `10px 20px` | ✅ 完整（padding/margin 1-4 值） |

### 1.3 CSS 属性 → GPUI 映射覆盖率

**文件：** `crates/engine/src/css/mapper.rs`

#### 已支持的 CSS 属性（完整清单）

| 分类 | CSS 属性 | GPUI 方法 | 支持度 |
|------|---------|-----------|--------|
| **盒模型** | `width` | `.w(gpui::px(N))` | ✅ 完整 |
| | `height` | `.h(gpui::px(N))` | ✅ 完整 |
| | `padding`（1-4 值简写） | `.p()` / `.py().px()` / `.pt().px().pb()` / `.pt().pr().pb().pl()` | ✅ 完整 |
| | `padding-top/bottom/left/right` | `.pt()` / `.pb()` / `.pl()` / `.pr()` | ✅ 完整 |
| | `margin`（1-4 值简写） | `.m()` / `.my().mx()` / `.mt().mx().mb()` / `.mt().mr().mb().ml()` | ✅ 完整 |
| | `margin-top/bottom/left/right` | `.mt()` / `.mb()` / `.ml()` / `.mr()` | ✅ 完整 |
| | `border-radius` | `.rounded(gpui::px(N))` | ✅ 完整 |
| **文本** | `font-size` | `.text_size(gpui::px(N))` | ✅ 完整 |
| | `font-weight` | `.font_weight(gpui::FontWeight::*)` | ✅ 8 种关键字 |
| | `text-align` | `.text_left()` / `.text_center()` / `.text_right()` | ✅ 完整 |
| | `line-height` | `.line_height(gpui::px(N))` | ✅ 完整 |
| **Flexbox** | `display: flex/none` | `.flex()` / `.hidden()` | ✅ 基础 |
| | `flex-direction: row/column` | `.flex_row()` / `.flex_col()` | ✅ 基础 |
| | `justify-content` | `.justify_center/start/end/between()` | ✅ 5 种值 |
| | `align-items` | `.items_center/start/end()` | ✅ 3 种值 |
| | `flex: N` | 仅 `flex: 1` → `.flex_1()` | ⚠️ 仅数字 1 |
| | `min-width: 0` | `.min_w_0()` | ✅ |
| | `min-height: 0` | `.min_h_0()` | ✅ |
| | `gap` | `.gap(gpui::px(N))` | ✅ 完整 |
| | `max-width`/`max-height` | 无 | ❌ **不支持** |
| | `flex-wrap`/`flex-grow`/`flex-shrink`/`flex-basis` | 无 | ❌ **不支持** |
| | `align-self`/`align-content` | 无 | ❌ **不支持** |
| | `order` | 无 | ❌ **不支持** |
| **视觉效果** | `background`/`background-color` | `.bg(gpui::rgb(N))` | ✅ 完整（支持 var 运行时主题） |
| | `color` | `.text_color(gpui::rgb(N))` | ✅ 完整（支持 var 运行时主题） |
| | `opacity` | `.opacity(N)` | ✅ 完整 |
| | `overflow: hidden/scroll` | `.overflow_hidden()` / `.overflow_scroll()` | ✅ 基础 |
| | `overflow-x`/`overflow-y` | 无 | ❌ **不支持** |
| **边框** | `border`（简写） | 无 | ❌ **不支持** |
| | `border-width`/`border-style`/`border-color` | 无 | ❌ **不支持** |
| | `border-top/right/bottom/left` | 无 | ❌ **不支持** |
| | `outline` | 无 | ❌ **不支持** |
| **背景** | `background-image` | 无 | ❌ **不支持** |
| | `background-size`/`background-position` | 无 | ❌ **不支持** |
| | `background-repeat` | 无 | ❌ **不支持** |
| **定位** | `position`（absolute/relative/fixed） | 无 | ❌ **不支持** |
| | `top`/`right`/`bottom`/`left` | 无 | ❌ **不支持** |
| | `z-index` | 无 | ❌ **不支持** |
| **变换/动画** | `transform` | 无 | ❌ **不支持** |
| | `transition` | 无 | ❌ **不支持** |
| | `animation` | 无 | ❌ **不支持** |
| **其他** | `box-sizing` | 无 | ❌ **不支持** |
| | `white-space` | 无 | ❌ **不支持** |
| | `cursor` | 无 | ❌ **不支持** |
| | `display: block/inline/inline-block/grid` | 无 | ❌ **不支持** |
| | `visibility` | 无 | ❌ **不支持** |
| | `box-shadow`/`text-shadow` | 无 | ❌ **不支持** |
| | `font-family` | 无 | ❌ **不支持** |
| | `font-style` | 无 | ❌ **不支持** |
| | `text-decoration` | 无 | ❌ **不支持** |
| | `letter-spacing`/`word-spacing` | 无 | ❌ **不支持** |
| | `list-style` | 无 | ❌ **不支持** |

### 1.4 补充：RML 特有布局属性（非 CSS 标准）

这些属性在 RML 中通过 `apply_static_attr` 或元素级处理，不属于 CSS 标准：

| 属性 | 使用方式 | 对应 GPUI 方法 | 说明 |
|------|---------|---------------|------|
| `v-flex=""` | 元素属性 | `.v_flex()` | 垂直 flex 布局快捷方式 |
| `h-flex=""` | 元素属性 | `.h_flex()` | 水平 flex 布局快捷方式 |
| `class="..."` | 元素属性 | CSS 类名匹配 | 触发 `apply_css_styles` |
| `id="..."` | 元素属性 | CSS ID 匹配 | 触发 `apply_css_styles` |
| `style="..."` | 元素属性 | 内联 CSS 映射 | 触发 `apply_inline_style` |
| `ref="..."` | 指令 | 稳定 ID 生成 | 用于事件绑定引用 |

### 1.5 CSS 覆盖率统计

| 维度 | 计数 | 占比 |
|------|------|------|
| CSS 标准属性总数（常用） | ~60 | 100% |
| 已映射属性 | 22 | ~37% |
| 未支持属性 | ~38 | ~63% |
| CSS 选择器类型总数（常用） | ~15 | 100% |
| 已支持选择器类型 | 9 | ~60% |
| 已支持值类型（完整映射） | 6/12 | ~50% |

**关键差距总结：**

1. **定位系统缺失** — 无 `position`/`top`/`right`/`bottom`/`left`/`z-index`，无法实现绝对定位、固定定位、层叠
2. **边框系统缺失** — 无 `border`/`outline`，`border-radius` 是唯一支持的边框属性
3. **变换与动画缺失** — 无 `transform`/`transition`/`animation`
4. **Flexbox 不完整** — 缺少 `flex-wrap`/`flex-grow`/`flex-shrink`/`flex-basis`/`align-self`/`align-content`
5. **伪类伪元素缺失** — 无 `:hover`/`:focus`/`::before`/`::after`，无法实现交互态样式
6. **属性选择器缺失** — 无 `[attr]` 选择器
7. **盒模型不完整** — 无 `box-sizing`/`max-width`/`max-height`
8. **溢出控制不完整** — 无 `overflow-x`/`overflow-y`
9. **字体不完整** — 无 `font-family`/`font-style`/`text-decoration`/`letter-spacing`

---

## 二、组件特定属性

### 2.1 属性分类体系

RML 组件属性分为三类，统一在 `props_registry.rs` 和 `component.rs` 中管理：

| 属性类型 | 声明方式 | 注册位置 | 覆盖范围 |
|---------|---------|---------|---------|
| **通用静态属性** | `label="..."` / `primary=""` | `COMMON_STATIC_PROPS` | 所有 Stateless/Stateful 组件 |
| **通用绑定属性** | `value={field}` | `COMMON_BIND_PROPS` | 所有 Stateless/Stateful 组件 |
| **通用事件属性** | `on-click={fn}` | `COMMON_EVENT_PROPS` | 所有 Stateless/Stateful 组件 |
| **组件专用属性** | 因组件而异 | `COMPONENT_PROPS` + 各组件模块 | 仅特定组件 |

### 2.2 通用属性清单

#### 通用静态属性（所有组件共享）

| 属性名 | 说明 | 映射方法 |
|--------|------|---------|
| `label` | 文本标签 | `.label("...")` |
| `placeholder` | 占位文本 | `.placeholder("...")` |
| `tooltip` | 提示文本 | `.tooltip("...")` |
| `primary` / `secondary` / `danger` / `success` / `warning` / `info` / `ghost` / `link` / `text` | Button 变体 | `.primary()` 等 |
| `size` | 尺寸（xsmall/small/medium/large） | `.with_size(Size::*)` |
| `compact` | 紧凑模式 | `.compact()` |
| `loading` | 加载状态 | `.loading()` |
| `disabled` | 禁用状态（布尔） | `.disabled(true/false)` |
| `selected` | 选中状态（布尔） | `.selected(true/false)` |
| `font_thin` / `font_extralight` / `font_light` / `font_normal` / `font_medium` / `font_semibold` / `font_bold` / `font_extrabold` / `font_black` | 字体权重 | `.font_bold()` 等 |
| `h_flex` / `v_flex` | 布局方向 | `.h_flex()` / `.v_flex()` |

#### 通用绑定属性（所有组件共享）

| 属性名 | 说明 | 映射方法 |
|--------|------|---------|
| `content={expr}` | 直接嵌入元素 | `.child(expr)` |
| `value={expr}` | 值绑定 | `.value(expr.clone())` |
| `disabled={expr}` | 禁用状态绑定 | `.disabled(expr)` |
| `selected={expr}` | 选中状态绑定 | `.selected(expr)` |
| `checked={expr}` | 勾选状态绑定 | `.selected(expr)` |
| `label={expr}` | 文本标签绑定 | `.label(expr.clone())` |
| `size={expr}` | 尺寸绑定 | `.with_size(expr)` |

#### 通用事件属性（所有组件共享）

| 属性名 | 说明 | 映射方法 | 事件参数 |
|--------|------|---------|---------|
| `on-click={fn}` | 点击事件 | `.on_click(cx.listener(...))` | `ClickEvent` → `rml_convert::from_gpui_click` |

### 2.3 各组件专用属性清单

#### Button（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `label` | 通用 | 按钮文本 | ✅ |
| `primary` / `secondary` / `danger` / `success` / `warning` / `info` / `ghost` / `link` / `text` | 通用 | 变体 | ✅ |
| `size` | 通用 | 尺寸 | ✅ |
| `disabled` / `selected` | 通用 | 状态 | ✅ |
| `loading` / `compact` | 通用 | 修饰 | ✅ |
| `on-click` | 通用 | 点击事件 | ✅ |
| 文本子节点 | — | 作为 label | ✅ |

**专用属性：** 无（完全依赖通用属性集）

---

#### Input / TextInput（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `placeholder` | 通用 | 占位文本 | ✅ |
| `disabled` | 通用 | 禁用 | ✅ |
| `on-change` | **专用** | 输入变化事件 | ✅ |
| `model={field}` | 指令 | 双向绑定 | ✅ |
| `size` | 通用 | 尺寸 | ✅ |

**注意事项：**
- Input 是 `Stateful` 组件，需要视图字段 `Entity<InputState>`
- `model` 指令由 `binding.rs` 独立处理，不走组件属性系统
- `on-change` 事件参数为 `rml_ui::InputState`

---

#### Accordion（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `multiple` | **专用** | 允许多个同时展开 | ✅ |
| `bordered` | **专用** | 带边框 | ✅ |
| `on-toggle-click` | **专用** | 切换事件 | ✅ |
| `size` | 通用 | 尺寸 | ✅ |

**子项（AccordionItem / item / accordion-item）：**

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `title` | **专用** | 标题 | ✅ |
| `open` | **专用** | 是否展开 | ✅ |
| `icon` | **专用** | 图标名称 | ✅ |
| `disabled` | 通用 | 禁用 | ✅ |

---

#### TabBar / tab-bar（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `selected-index` | **专用** | 选中索引 | ✅ |
| `on-click` | **专用** | 点击事件 | ✅ |
| `prefix` / `suffix` | **专用** | 前缀/后缀元素 | ✅ |
| `last-empty-space` | **专用** | 末尾留空 | ⚠️ 已注册但 codegen 映射待确认 |
| `menu` | **专用** | 关联菜单 | ✅ |
| `track-scroll` | **专用** | 滚动追踪 | ⚠️ 已注册但 codegen 映射待确认 |
| `underline` / `pill` / `flat` / `outline` / `segmented` | **专用** | 变体快捷方法 | ✅ |

**子项（Tab / tab / TabItem / tab-item）：**

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `label` | 通用（Tab 专用） | 标签文本 | ✅ |
| `icon` | **专用** | 图标 | ✅ |
| `disabled` | 通用 | 禁用 | ✅ |
| `selected` | 通用 | 选中 | ✅ |
| `prefix` / `suffix` | **专用** | 前缀/后缀 | ✅ |
| `on-click` | 通用 | 点击事件 | ✅ |
| `closable` | **专用** | 可关闭 | ✅ |
| `underline` / `pill` / `flat` / `outline` / `segmented` | **专用** | 变体 | ✅ |
| TabItem: `title` / `title-icon` | **专用** | TabItem 标题 | ✅ |
| TabItem: `disabled` / `on-click` / `closable` | **专用** | TabItem 属性 | ✅ |

---

#### Table / table（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `columns` | **专用** | 列定义 | ✅ |
| `rows` | **专用** | 行数据 | ✅ |
| `delegate` | **专用** | 委托 | ✅ |
| `bordered` / `borderless` | **专用** | 边框模式 | ✅ |
| `stripe` | **专用** | 斑马纹 | ✅ |

**子项（Column / column）：**

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `key` | **专用** | 字段键 | ✅ |
| `title` | **专用** | 列标题 | ✅ |
| `width` | **专用** | 列宽 | ✅ |
| `align` | **专用** | 对齐 | ✅ |
| `field` | **专用** | 字段名 | ✅ |

---

#### DescriptionList / descriptions（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `vertical` | **专用** | 垂直模式 | ✅ |
| `horizontal` | **专用** | 水平模式 | ✅ |
| `bordered` | **专用** | 带边框 | ✅ |
| `columns` | **专用** | 列数 | ✅ |
| `label-width`（RML 中→`label_width`） | **专用** | 标签宽度 | ✅ |
| `items` | **专用** | items 数据绑定 | ✅（`<descriptions items={...}>`） |

**子项（DescriptionItem / description / DescriptionSeparator / separator）：**

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `label` | **专用** | 标签文本 | ✅ |
| `value` | **专用** | 值内容 | ✅ |
| `span` | **专用** | 跨列数 | ✅ |

---

#### Card（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `title` | **专用** | 卡片标题 | ✅ |
| `extra` | **专用** | 额外内容 | ✅ （已注册但 codegen 映射需确认） |
| `cover` | **专用** | 封面 | ✅ （已注册但 codegen 映射需确认） |
| `footer` | **专用** | 底部 | ✅ （已注册但 codegen 映射需确认） |
| `bordered` / `borderless` | **专用** | 边框模式 | ✅ |
| `hoverable` | **专用** | 悬浮效果 | ✅ |

---

#### Avatar（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `src` | **专用** | 图片 URL | ✅ |
| `name` | **专用** | 显示名称 | ✅ |
| `placeholder` | **专用** | 占位图标（IconName 枚举） | ✅ |
| `size` | 通用 | 尺寸 | ✅ |
| 文本子节点 | — | 作为 name | ✅ |

---

#### AvatarGroup（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `limit` | **专用** | 显示数量上限 | ✅ |
| `ellipsis` | **专用** | 超出省略 | ✅ |

---

#### Tree（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `on-activate` | **专用** | 激活事件 | ✅ |
| `on-select` | **专用** | 选中事件 | ✅ |

**注意事项：** Tree 是 `Stateful` 组件，数据由 `TreeState` Entity 通过 `ref` 指令提供，不支持 `items` 绑定

---

#### MenuBar / menu-bar / menu（gpui-component）

| 属性 | 类型 | 说明 | 完成度 |
|------|------|------|--------|
| `items` | **专用** | 菜单数据绑定 | ✅ |

**说明：** MenuBar 支持 `items={menu_items}` 数据绑定，子节点也支持声明式 `<menu-item>` / `<menu-separator>` 子标签。

---

#### Badge / Checkbox / Label / Separator / Tag / Progress / ProgressCircle / Slider / Switch

这些组件仅支持通用属性集（`label`、`disabled`、`selected`、`size`、`on-click` 等），无专用属性。

---

### 2.4 属性覆盖率统计

| 维度 | 计数 | 说明 |
|------|------|------|
| 组件总数 | 23 | 含内置 HTML 标签 |
| 通用属性 | 22 | 静态 17 + 绑定 7 + 事件 1（有重叠） |
| 专用属性（已注册） | 58 | COMPONENT_PROPS 中注册 |
| 专用属性（已实现映射） | 52+ | 约 90% 已实现 |
| 未实现映射（已注册） | 约 6 | `extra`/`cover`/`footer`/`last_empty_space`/`track_scroll` 等 |

**关键差距：**
- Card 的 `extra`/`cover`/`footer` 已注册到 `COMPONENT_PROPS`，但 `component_bind_setter` 中映射需确认是否存在
- `last-empty-space` 和 `track-scroll` 已注册但 codegen setter 映射可能缺失
- 部分专用属性仅支持 `static` 形式，不支持 `bind` 形式（需要通过组件模块扩展）

---

## 三、已支持组件清单

### 3.1 内置 HTML 标签（原生轨）

| 标签 | GPUI 构造 | 默认样式 | 自闭合 | 完成度 |
|------|-----------|---------|--------|--------|
| `div` | `gpui::div()` | 无 | 否 | ✅ 完整 |
| `span` | `gpui::div()` | 无（同 div） | 否 | ✅ 完整 |
| `p` | `gpui::div()` | `.text_sm().text_color(...)` | 否 | ✅ 完整 |
| `h1`~`h6` | `gpui::div()` | `.text_size(px(N))` | 否 | ✅ 完整（h2 值与其他不一致） |
| `button` | `gpui::div()` | 无 | 否 | ⚠️ 原生轨无交互，建议用 `<Button>` |
| `input` | `gpui::div()` | 无 | 是 | ⚠️ 原生轨无交互，建议用 `<Input>` |
| `textarea` | `gpui::div()` | 无 | 是 | ⚠️ 同上 |
| `ul` / `ol` | `gpui::div().flex().flex_col()` | 垂直排列 | 否 | ✅ |
| `li` | `gpui::div()` | 无 | 否 | ✅ |
| `img` | `gpui::div()` | 无 | 是 | ⚠️ 原生轨无图片加载能力 |
| `a` | `gpui::div()` | 无 | 否 | ⚠️ 原生轨无链接能力 |
| `label` | `gpui::div()` | 无 | 否 | ✅ |
| `br` | `gpui::div().hidden()` | 零尺寸占位 | 是 | ✅ |
| `code` | `gpui::div()` | 无 | 否 | ✅ |

**原生轨评估：** 所有内置标签都缺乏浏览器默认行为和交互能力（`button` 无点击、`input` 无输入、`img` 无图片加载、`a` 无导航）。这些标签的本质角色是 **语义化的样式容器**，真实交互需通过扩展组件实现。

---

### 3.2 扩展组件（gpui-component 路由表）

#### 完整度定义

| 等级 | 含义 |
|------|------|
| ⭐⭐⭐ | 声明式属性覆盖完整、子节点/插槽支持完善、demo 案例覆盖 |
| ⭐⭐ | 核心功能可用、部分次要属性缺失、有 demo 案例 |
| ⭐ | 基础功能可用、属性覆盖有限、无 demo 案例或仅基础使用 |

| 组件 | 标签名（别名） | 类型 | 容器 | 完成度 | 质量评估 |
|------|---------------|------|------|--------|---------|
| **Button** | `Button` | Stateless | 否 | ⭐⭐⭐ | 完整。支持所有变体/尺寸/状态/事件，有 demo |
| **Accordion** | `Accordion` / `accordion` | StatelessWithItems | 否 | ⭐⭐⭐ | 完整。支持 multiple/bordered/item 属性，有 demo |
| **TabBar** | `TabBar` / `tab-bar` | StatelessWithItems | 否 | ⭐⭐⭐ | 完整。支持 5 种变体/图标/菜单/closable，有 demo |
| **Table** | `Table` / `table` | StatelessWithItems | 否 | ⭐⭐⭐ | 完整。支持列定义/行数据/模板插槽/边框/斑马纹，有 demo |
| **DescriptionList** | `DescriptionList` / `descriptions` | StatelessWithItems | 否 | ⭐⭐⭐ | 完整。支持 vertical/bordered/columns/label_width/items 绑定，有 demo |
| **Card** | `Card` | Stateless | 是 | ⭐⭐ | 主要属性可用（title/hoverable/bordered），`extra`/`cover`/`footer` 绑定映射待确认，有 demo |
| **Avatar** | `Avatar` | StatelessNoId | 否 | ⭐⭐ | 支持 src/name/placeholder/size，有 demo |
| **AvatarGroup** | `AvatarGroup` | StatelessNoId | 是 | ⭐⭐ | 支持 limit/ellipsis，有 demo |
| **Input** | `Input` / `TextInput` | Stateful | 否 | ⭐⭐⭐ | 完整。支持 model 双向绑定/onchange/validate/converter，有 demo |
| **Badge** | `Badge` | StatelessNoId | 是 | ⭐ | 基本可用，无独立 demo |
| **Checkbox** | `Checkbox` | Stateless | 否 | ⭐ | 基本可用，无独立 demo |
| **Label** | `Label` | Stateless | 否 | ⭐ | 基本可用，无独立 demo |
| **Separator** | `Separator` | Stateless | 否 | ⭐ | 基本可用，无独立 demo |
| **Tag** | `Tag` | Stateless | 否 | ⭐ | 基本可用，无独立 demo |
| **Progress** | `Progress` | Stateless | 否 | ⭐ | 基本可用，无独立 demo |
| **ProgressCircle** | `ProgressCircle` | Stateless | 否 | ⭐ | 基本可用，无独立 demo |
| **Slider** | `Slider` | Stateless | 否 | ⭐ | 基本可用，无独立 demo |
| **Switch** | `Switch` | Stateless | 否 | ⭐ | 基本可用，无独立 demo |
| **Tree** | `Tree` | Stateful | 否 | ⭐⭐ | 需要 TreeState Entity，支持 on_activate/on_select，在 activity_panel 中使用 |
| **CodeEditor** | `CodeEditor` | Stateful | 否 | ⭐⭐ | 基于 Input 的代码编辑器，自动应用 mono 字体，在 LSP demo 中使用 |
| **TitleBar** | `TitleBar` | StatelessNoId | 是 | ⭐ | 基本可用，用于窗口自定义标题栏 |
| **NativeStatusBar** | `NativeStatusBar` / `native-status-bar` | StatelessNoId | 是 | ⭐ | 基本可用，用于窗口状态栏 |
| **MenuBar** | `MenuBar` / `menu-bar` / `menu` | Stateless | 否 | ⭐⭐⭐ | 完整。支持声明式子标签和 items 数据绑定，5 个 demo 案例 |
| **ActivityBar** | `ActivityBar` | EntityRef | 否 | ⭐⭐ | 需要 Entity 字段引用，在 shell 中使用 |
| **ButtonGroup** | `ButtonGroup` | Stateless | 是 | ⭐ | 基本可用，无独立 demo |

---

### 3.3 Builder 子项标签

| 标签 | 父组件 | 属性 | 完成度 |
|------|--------|------|--------|
| `AccordionItem` / `item` / `accordion-item` | Accordion | title/open/icon/disabled | ⭐⭐⭐ |
| `Tab` / `tab` | TabBar | label/icon/disabled/selected/prefix/suffix/on_click/closable + 变体 | ⭐⭐⭐ |
| `TabItem` / `tab-item` | TabBar | title/title_icon/disabled/on_click/closable | ⭐⭐ |
| `Column` / `column` | Table | key/title/width/align/field | ⭐⭐⭐ |
| `DescriptionItem` / `description` | DescriptionList | label/value/span | ⭐⭐⭐ |
| `DescriptionSeparator` / `separator` | DescriptionList | 无属性 | ⭐⭐⭐ |

---

### 3.4 菜单系统组件（由 compiler/menu/ 处理）

| 标签 | 父上下文 | 说明 | 完成度 |
|------|---------|------|--------|
| `context-menu` | 任意 | 右键菜单容器 | ⭐⭐⭐ |
| `dropdown-menu` | 任意 | 下拉菜单容器，支持 anchor/check-side/scrollable/max-h | ⭐⭐⭐ |
| `menu-bar` / `app-menu-bar` | 任意 | 菜单栏容器 | ⭐⭐⭐ |
| `menu-item` | 菜单容器 | 菜单项：label/icon/onclick/disabled/checked/header/href/command | ⭐⭐⭐ |
| `menu-separator` | 菜单容器 | 菜单分隔线 | ⭐⭐⭐ |
| `template slot="menu"` | 菜单容器 | scoped slot 模板 | ⭐⭐⭐ |

---

### 3.5 模板/容器特殊标签

| 标签 | 说明 | 完成度 |
|------|------|--------|
| `slot name="..."` | 插槽占位符（Vue 风格） | ⭐⭐⭐ |
| `template slot="name"` | 具名插槽内容载体 | ⭐⭐⭐ |
| `template slot field="scoped"` | scoped slot | ⭐⭐⭐ |
| `component content={expr}` | 透明容器嵌入动态元素 | ⭐⭐⭐ |
| `status-bar` | 状态栏容器 | ⭐⭐⭐ |

---

### 3.6 根标签

| 根标签 | 说明 | 完成度 |
|--------|------|--------|
| `window` | 基础窗口（透明标题栏） | ⭐⭐⭐ |
| `modern-window` | 自绘标题栏/菜单/状态栏 | ⭐⭐ |
| `tab-window` | TabBar 标题栏高级窗口 | ⭐⭐⭐ |
| `dialog` | 模态对话框 | ⭐⭐⭐ |
| `component` | 可复用组件根标签 | ⭐⭐⭐ |

---

### 3.7 完成度总体统计

| 类别 | 总数 | ⭐⭐⭐ 完整 | ⭐⭐ 良好 | ⭐ 基础 |
|------|------|-----------|---------|---------|
| 内置 HTML 标签 | 18 | 14 | 0 | 4（button/input/textarea/img/a 无交互） |
| 扩展组件 | 23 | 8 | 7 | 8 |
| Builder 子项 | 6 | 5 | 1 | 0 |
| 菜单系统 | 6 | 6 | 0 | 0 |
| 模板/容器 | 5 | 5 | 0 | 0 |
| 根标签 | 5 | 4 | 1 | 0 |
| **合计** | **63** | **42** | **9** | **12** |

**占比：** 完整度 ⭐⭐⭐ = 67%，良好 ⭐⭐ = 14%，基础 ⭐ = 19%

---

### 3.8 质量观察

#### 优势

1. **组件注册体系完善** — `tags.rs` 路由表 + `props_registry.rs` 属性注册表 + 验证器三重校验
2. **双轨制策略清晰** — 原生 HTML 标签（语义容器）与扩展组件（真实交互）分离
3. **声明式覆盖率高** — 主要交互组件（Button/Accordion/TabBar/Table/DescriptionList/Input）属性覆盖完整且有 demo 案例
4. **模板系统强大** — slot/scoped slot/each 迭代/条件渲染，接近 Vue 模板的灵活性
5. **菜单系统完整** — 上下文菜单、下拉菜单、菜单栏全部支持声明式和数据绑定
6. **类型安全** — 属性通过 `ComponentKind` 分派不同构造策略，编译期检查

#### 待改进

1. **无独立 demo 的组件** — Badge/Checkbox/Label/Separator/Tag/Progress/ProgressCircle/Slider/Switch/ButtonGroup/TitleBar/NativeStatusBar 均无独立 demo
2. **组件属性映射不完全** — Card 的 `extra`/`cover`/`footer` 已注册但映射可能缺失
3. **CSS 支持范围有限** — 仅支持 ~37% CSS 标准属性，无定位/边框/变换/动画等
4. **原生轨标签交互缺失** — 原生 `button`/`input`/`img`/`a`/`textarea` 仅作为 `div` 样式容器，无交互行为
5. **无内置 Loading/Empty/Error 状态组件** — 缺乏骨架屏、空状态、错误提示等基础 UX 组件
6. **无表单布局组件** — 缺乏 Form/FormItem/Radio/RadioGroup/DatePicker/Select 等标准表单组件
7. **无 DataDisplay 组件** — 缺乏 Tooltip/Popover/Popconfirm/Drawer/Modal/Notification/Timeline/List 等
8. **无 Navigation 组件** — 缺乏 Breadcrumb/Dropdown/Pagination/Steps/Tabs 导航类（TabBar 除外）
9. **无 Feedback 组件** — 缺乏 Alert/Message/Notification/Progress（仅进度条）/Spin/Drawer

---

## 四、总结

### 4.1 通用属性配置

- **CSS 标准属性覆盖率：** ~37%（22/60 常用属性）
- **选择器覆盖率：** ~60%（9/15 常用选择器类型）
- **主要差距：** 定位、边框、变换动画、Flexbox 不完整、伪类、属性选择器
- **特性支持：** 主题变量（var 运行时查询）、内联样式、CSS 类名/ID 匹配

### 4.2 组件特定属性

- **注册体系完善：** COMMON_PROPS（通用）+ COMPONENT_PROPS（专用）+ SHELL_PROPS（窗口）
- **通用属性：** 22 个（跨越所有组件）
- **专用属性：** 58 个（在 COMPONENT_PROPS 注册）
- **实现率：** ~90% 的已注册属性已实现 codegen 映射
- **主要差距：** 少数属性注册后映射缺失，部分仅支持 static 不支持 bind

### 4.3 组件体系

- **总组件数：** 63 个（含内置标签、扩展组件、Builder 子项、菜单、模板、根标签）
- **扩展组件（核心交互）：** 23 个（来自 gpui-component）
- **完整度：** 67% 完整 ⭐⭐⭐，14% 良好 ⭐⭐，19% 基础 ⭐
- **亮点：** Button/Accordion/TabBar/Table/DescriptionList/Input/Menu 均为完整覆盖
- **不足：** 缺乏 Form/DataDisplay/Feedback/Navigation 等标准 UI 组件类别

---

## 五、CSS 分层体系方案（规划）

### 5.1 三层 CSS 架构

| 层级 | 名称 | 注册方式 | 作用域 | 优先级 | 当前状态 |
|------|------|---------|--------|--------|---------|
| **Layer 1** | 应用层 CSS | `Builder::with_style("path.css")` | 全局所有组件 | 低 | ✅ 已实现（可多个文件，合并为全局 StyleSheet） |
| **Layer 2** | 页面层 CSS | RML 中 `<style source="/button.css"/>` | 当前 `.rml` 组件及其子组件树 | 中 | ❌ 需实现 |
| **Layer 3** | 组件级内联 CSS | `style="prop: val;"` 属性 | 当前元素 | 高 | ✅ 已实现 |

### 5.2 应用层 CSS（Layer 1）— 当前机制

**文件：** `crates/engine/src/build/mod.rs`

- 通过 `Builder::with_style(...)` 注册，支持多次调用多个文件
- 自动扫描 `assets/` 根目录下 `.css` 文件（排除 themes/ 子目录）
- 所有文件合并为一个全局 `StyleSheet` → 注入 `CodegenCtx.stylesheet`
- 编译期解析为 AST，class/id 匹配时生成 GPUI 方法调用代码
- `:root` 变量：后注册的覆盖前者

**当前问题：**
- 不支持作用域隔离，所有规则在全局平面匹配
- 不支持按组件分组管理样式

### 5.3 页面层 CSS（Layer 2）— 规划方案

#### 5.3.1 RML 语法

```rml
<component>
    <!-- 页面级样式引入（作用域：当前组件及其子组件树） -->
    <style source="/button.css"/>
    <style source="/table.css"/>

    <!-- 页面级内联样式 -->
    <style>
        .custom-btn { background: var(--primary); padding: 8px 16px; }
        .custom-table { border-radius: 4px; }
    </style>

    <div class="custom-btn">
        <Button label="Submit"/>
    </div>
</component>
```

#### 5.3.2 实现路径

| 步骤 | 模块 | 变更内容 |
|------|------|---------|
| **Step 1** | Parser (`parser/`) | 新增 `<style>` 标签识别：`source` 属性 → 外部 CSS 文件引用；文本子节点 → 内联 CSS 源码 |
| **Step 2** | AST (`parser/ast.rs`) | 新增 `SpecialElement::Style(StyleElement)` 变体或 Element 标记字段 |
| **Step 3** | CodegenCtx (`compiler/mod.rs`) | 新增 `page_stylesheet: Option<StyleSheet>` 字段，持有当前组件的页面级样式表 |
| **Step 4** | Compiler (`compiler/mod.rs`) | `compile()` 中：解析 `<style>` 子节点 → `css::parse()` 解析 → 注入 `CodegenCtx.page_stylesheet` |
| **Step 5** | Matcher (`css/matcher.rs`) | `collect_matching_declarations` 支持多级样式表：先查 page_stylesheet（高优先级），再查全局 stylesheet（低优先级） |
| **Step 6** | Codegen (`codegen/node.rs`) | `gen_element` 中传递 page_stylesheet 给 `apply_css_styles` |
| **Step 7** | Builder (`build/mod.rs`) | 缓存机制需考虑页面级 CSS 变化；支持构建期解析 `<style source>` 引用的外部 CSS 文件 |
| **Step 8** | Scoping（可选） | 通过 CSS 选择器前缀或 Shadow DOM 模拟实现样式隔离 |

#### 5.3.3 优先级叠加规则

```
最终生效样式 = Layer 1（全局） + Layer 2（页面，同名覆盖 Layer 1）+ Layer 3（内联，最高优先级）
```

- 选择器匹配顺序：先匹配 Layer 2 + Layer 3（页面/内联），再匹配 Layer 1（全局）
- 同层后出现的规则覆盖先出现的
- `!important` 暂不支持（可后续扩展）

#### 5.3.4 文件引用解析

`<style source="/button.css">`：
- 路径相对于 `assets/` 根目录
- 构建期由 Builder 读取并解析为 StyleSheet
- 支持热重载（开发期监听文件变化）

---

## 六、综合组件覆盖规划

### 6.1 gpui-component 完整能力 vs RML 当前覆盖

**文件：** gpui-component v0.5.2 (`C:\Users\lusid\.cargo\git\checkouts\gpui-component-95ce574d8a0da8b8\063e55b`)

| 分类 | gpui-component 可用组件 | RML 已集成 | RML 未集成 |
|------|-----------------------|-----------|-----------|
| **DataDisplay** | 13 | 7 | 6（Link, Rating, Collapsible, HoverCard, List, Popover） |
| **Form** | 9 | 3 | 6（Calendar, DatePicker, ColorPicker, Combobox, InputNumber, OTPInput） |
| **Navigation** | 6 | 1 | 5（Breadcrumb, Pagination, Stepper, Sidebar, Dock） |
| **Feedback** | 4 | 1 | 3（Alert, Spinner, Skeleton） |
| **Layout** | 6 | 0 | 6（Resizable, Scroll, GroupBox, VirtualList, WindowBorder） |
| **弹出/浮层** | 3 | 0 | 3（Sheet, FocusTrap, SearchableList） |
| **数据可视化** | 2 | 0 | 2（Chart, Plot） |
| **其他** | 6 | 0 | 6（Highlighter, Text, Animation, History, Setting, NativeMenu） |
| **合计** | **~49** | **12** | **~37** |

**备注：** 另有约 12 个组件（Dialog/Form/List/Popover/Radio/Select/Tooltip/Notification/Icon/Kbd 等）已 re-export 但未在 tags.rs 注册，属于"半集成"状态。

### 6.2 组件集成优先级规划

#### Phase 1：高优先级（直接从 re-export 推进到 tags.rs 注册 + demo）

这些组件已 re-export 到 `rml_ui`，只需在 `tags.rs` 注册 + codegen `component.rs` 添加映射 + demo 案例：

| 组件 | 当前状态 | 推进步骤 |
|------|---------|---------|
| **Tooltip** | 已 re-export | tags.rs 注册为 Stateless + tooltip 属性完善 |
| **Popover** | 已 re-export | tags.rs 注册为 StatelessWithItems + codegen 模块 |
| **List** | 已 re-export | tags.rs 注册为 StatelessWithItems + Column 式 builder |
| **Radio** | 已 re-export | tags.rs 注册为 Stateless + RadioGroup 分组 |
| **Select** | 已 re-export | tags.rs 注册为 Stateless + items 数据绑定 |
| **Form** | 已 re-export | tags.rs 注册为容器 + FormItem 校验 |
| **AlertDialog** | rml_ui 自定义 | 增强属性映射 + 完善 demo |
| **Notification** | 已 re-export | tags.rs 注册 + Root 集成 |

#### Phase 2：中优先级（需新增 re-export + tags.rs 注册）

| 组件 | gpui-component 模块 | 类型估计 | 备注 |
|------|--------------------|---------|------|
| **Breadcrumb** | `breadcrumb.rs` | Stateless | 导航路径 |
| **Pagination** | `pagination.rs` | Stateless | 分页控制 |
| **Stepper** | `stepper/` | StatelessWithItems | 步骤向导 |
| **Alert** | `alert.rs` | StatelessNoId | 提示条 |
| **Spinner** | `spinner.rs` | StatelessNoId | 加载旋转器 |
| **Skeleton** | `skeleton.rs` | StatelessNoId | 骨架屏 |
| **Link** | `link.rs` | Stateless | 超链接 |
| **Rating** | `rating.rs` | Stateless | 评分 |
| **Collapsible** | `collapsible.rs` | Stateless | 可折叠面板 |
| **HoverCard** | `hover_card.rs` | Stateless | 悬停卡片 |

#### Phase 3：低优先级（复杂交互组件）

| 组件 | gpui-component 模块 | 备注 |
|------|--------------------|------|
| **Dock** | `dock/` | 可拖拽停靠面板，与 TabWindow 深度集成 |
| **Sidebar** | `sidebar/` | 侧边栏框架 |
| **Combobox** | `combobox.rs` | 下拉组合框 |
| **Calendar / DatePicker** | `time/` | 日期选择 |
| **ColorPicker** | `color_picker.rs` | 颜色选择器 |
| **InputNumber** | `input/number_input.rs` | 数字输入 |
| **OTPInput** | `input/otp_input.rs` | 一次性密码输入 |
| **Resizable** | `resizable/` | 可拖拽调整面板 |
| **Scroll** | `scroll/` | 滚动区域 |
| **VirtualList** | `virtual_list.rs` | 虚拟化列表 |
| **Sheet** | `sheet.rs` | 底部弹出面板 |
| **SearchableList** | `searchable_list/` | 可搜索列表 |
| **Chart / Plot** | `chart/`、`plot/` | 图表套件 |
| **Text Editor** | `text/` | 富文本编辑器 |
| **Highlighter** | `highlighter/` | 语法高亮 |
| **Setting** | `setting/` | 设置页面框架 |

### 6.3 组件集成模式标准

所有新组件集成需遵循以下标准流程：

| 步骤 | 文件 | 操作 |
|------|------|------|
| 1. re-export | `crates/ui/src/lib.rs` | `pub use gpui_component::ComponentName;` |
| 2. 路由表注册 | `crates/engine/src/tags.rs` | `component_lookup()` 添加条目 + `ComponentKind` 选择 |
| 3. 属性注册 | `crates/engine/src/compiler/props_registry.rs` | `COMPONENT_PROPS` 添加专用属性 |
| 4. Codegen 映射 | `crates/engine/src/compiler/component.rs` | `component_static_setter` / `component_bind_setter` / `component_event_setter` 添加分支 |
| 5. 验证器 | `crates/engine/src/compiler/validator.rs` | 添加标签/属性合法性验证 |
| 6. Demo 案例 | `demo/src/cases/xxx_case.rml` + `.rml.rs` | 功能覆盖 demo |
| 7. 单元测试 | `crates/engine/src/compiler/component.rs` | gen_component 测试用例 |

---

## 七、CSS 体系完整规划

### 7.1 CSS 标准属性扩展路线图

基于当前 37% 覆盖率的 CSS 支持，按优先级规划扩展：

#### P0（高优先级，布局必需）

| CSS 属性 | GPUI 方法 | 实现难度 | 备注 |
|---------|-----------|---------|------|
| `max-width` / `max-height` | `.max_w()` / `.max_h()` | 低 | gpui Styled 已支持 |
| `flex-wrap` | `.flex_wrap()` | 低 | gpui Styled 已支持 |
| `flex-grow` / `flex-shrink` | `.flex_grow()` / `.flex_shrink()` | 低 | gpui Styled 已支持 |
| `flex-basis` | `.flex_basis()` | 低 | gpui Styled 已支持 |
| `align-self` | `.self_center/start/end()` | 低 | gpui Styled 已支持 |
| `align-content` | `.content_center/start/end()` | 低 | gpui Styled 已支持 |
| `overflow-x` / `overflow-y` | `.overflow_x()` / `.overflow_y()` | 低 | 需检查 gpui 支持 |
| `box-sizing` | — | 低 | GPUI 默认 border-box |
| `display: block` / `inline` | — | 低 | GPUI 默认 block |

#### P1（中优先级，UI 精细控制）

| CSS 属性 | GPUI 方法 | 实现难度 | 备注 |
|---------|-----------|---------|------|
| `border`（简写） | `.border()` / `.border_color()` | 中 | 需解析 border 简写 |
| `border-width` | `.border_width()` | 低 | gpui Styled 已支持 |
| `border-color` | `.border_color()` | 低 | gpui Styled 已支持 |
| `border-style` | — | 中 | GPUI 边框样式有限 |
| `outline` | `.outline()` | 低 | gpui Styled 已支持 |
| `cursor` | `.cursor()` | 低 | gpui Styled 已支持 |
| `text-decoration` | — | 中 | 需 GPUI 支持 |
| `letter-spacing` | `.letter_spacing()` | 低 | gpui Styled 已支持 |
| `font-family` | — | 中 | 需字体注册 |
| `font-style` | `.italic()` | 低 | gpui Styled 已支持 |

#### P2（低优先级，视觉增强）

| CSS 属性 | GPUI 方法 | 实现难度 | 备注 |
|---------|-----------|---------|------|
| `position: absolute/relative` | — | 高 | GPUI 无定位系统 |
| `top`/`right`/`bottom`/`left` | — | 高 | 依赖 position |
| `z-index` | — | 高 | 依赖 GPUI 渲染层 |
| `transform` | — | 高 | 需 GPUI 支持 |
| `transition` / `animation` | — | 高 | 需 GPUI 动画 API |
| `box-shadow` / `text-shadow` | — | 高 | GPUI 无阴影系统 |
| `background-image` | — | 高 | 需 GPUI 图片渲染 |
| `background-size` / `position` | — | 中 | 扩展 bg 映射 |
| `white-space` | — | 中 | 需 GPUI 文本支持 |
| `visibility` | — | 低 | 映射到 `hidden()`/`visible()` |

### 7.2 CSS 选择器扩展路线图

| 选择器 | 实现难度 | 备注 |
|--------|---------|------|
| `:hover` / `:focus` 伪类 | 高 | 需 GPUI 交互状态感知 |
| `:first-child` / `:last-child` / `:nth-child` | 中 | 需 codegen 传递子节点索引 |
| `:disabled` / `:checked` | 中 | 需组件状态暴露 |
| `::before` / `::after` 伪元素 | 高 | GPUI 无伪元素概念 |
| `[attr]` 属性选择器 | 低 | 解析器 + 匹配器扩展 |
| `+` 相邻兄弟选择器 | 中 | 需 codegen 传递兄弟信息 |
| `~` 通用兄弟选择器 | 中 | 同上 |

### 7.3 CSS 运行时体系

| 能力 | 当前状态 | 规划 |
|------|---------|------|
| 主题变量（`var(--primary)` 运行时查询） | ✅ 已实现 | 维持 |
| 非颜色变量（构建期内联 `var(--spacing)`） | ✅ 已实现 | 维持 |
| 多主题切换 | ✅ 已实现 | 维持 |
| CSS 变量 fallback | ✅ 已实现 | 维持 |
| 命名颜色 | ✅ 11 种基础 | 扩展至 CSS 标准 140+ 命名颜色 |
| `rgb()` / `rgba()` 函数 | ✅ 解析 | 完善内联映射 |
| `calc()` 函数 | ❌ 不支持 | 低优先级 |
| `media` 查询 | ❌ 不支持 | 桌面端优先级低 |
| `@supports` | ❌ 不支持 | 低优先级 |
| `@import` | ❌ 不支持 | 构建期替代方案 |

### 7.4 CSS 分层系统实现时序

| Phase | 内容 | 涉及模块 | 工作量估计 |
|-------|------|---------|-----------|
| **Phase A** | Layer 2 基础实现：Parser 识别 `<style>` + CodegenCtx 持有 page_stylesheet + Matcher 多级查询 | parser, compiler, css | 中（3-5 天） |
| **Phase B** | `<style source="..."/>` 文件引用支持 + 构建期解析 | build | 中（2-3 天） |
| **Phase C** | CSS 标准属性扩展 P0（max-w/h, flex-wrap/grow/shrink/basis, align-self/content, overflow-x/y） | css/mapper.rs | 小（1-2 天） |
| **Phase D** | CSS 标准属性扩展 P1（border/outline/cursor/letter-spacing/font-style/text-decoration） | css/mapper.rs | 中（2-3 天） |
| **Phase E** | 选择器扩展（属性选择器 + 伪类位置选择器） | css/parser.rs, css/matcher.rs | 中（2-3 天） |
| **Phase F** | CSS 标准属性扩展 P2（定位/变换/动画/阴影） | 全栈 | 大（待评估） |
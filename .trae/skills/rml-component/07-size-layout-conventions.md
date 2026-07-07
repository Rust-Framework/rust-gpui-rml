# 07 尺寸与布局规范

## size 属性

`size` 属性统一控制组件尺寸，使用 `medium` 不用 `middle`：

| 值 | 生成代码 | 说明 |
|----|----------|------|
| `xsmall` | `.with_size(rml_ui::Size::XSmall)` | 超小 |
| `small` | `.with_size(rml_ui::Size::Small)` | 小 |
| `medium` | `.with_size(rml_ui::Size::Medium)` | 中（**不用 middle**） |
| `large` | `.with_size(rml_ui::Size::Large)` | 大 |

**语法**：
```xml
<Button size="small">Small</Button>
<Button size="medium">Medium</Button>
<Button size="large">Large</Button>
<Button size={size_value}>Dynamic</Button>
```

**反模式**：
```xml
<!-- ❌ 使用 middle -->
<Button size="middle">Medium</Button>
```

## vertical 属性

`vertical` 控制布局方向，**默认横向**，仅 `vertical=true` 表示纵向：

| 写法 | 生成代码 | 说明 |
|------|----------|------|
| `vertical` | `.layout(gpui::Axis::Vertical)` | 纵向（静态） |
| `vertical="true"` | `.layout(gpui::Axis::Vertical)` | 纵向（静态） |
| `vertical="false"` | （无生成） | 横向（默认） |
| `vertical={is_vertical}` | `.layout(if self.is_vertical { Vertical } else { Horizontal })` | 动态 |

**设计原则**：不提供 `horizontal` 属性，默认横向。仅 `vertical=true` 切换纵向。

**适用组件**：DescriptionList（`.layout(gpui::Axis::*)`）

**反模式**：
```xml
<!-- ❌ 提供 horizontal -->
<DescriptionList horizontal={true}>
<!-- ❌ 同时提供 horizontal 和 vertical -->
<DescriptionList horizontal={true} vertical={false}>
```

## variant 快捷方法

Button/TabBar/Tab 支持 variant 快捷方法，值为空或 "true" 时启用：

```xml
<Button primary>Primary</Button>
<Button danger>Danger</Button>
<TabBar underline>Underline TabBar</TabBar>
<TabBar pill>Pill TabBar</TabBar>
```

| variant | 生成代码 |
|---------|----------|
| `primary` / `secondary` / `danger` / `success` / `warning` / `info` / `ghost` / `link` / `text` | `.primary()` 等 |
| `underline` / `pill` / `flat` / `outline` / `segmented` | `.underline()` 等 |

## 字体权重快捷方法

```xml
<div font-bold>Bold Text</div>
<div font-semibold>Semibold Text</div>
```

| 属性 | 生成代码 |
|------|----------|
| `font_thin` | `.font_thin()` |
| `font_extralight` | `.font_extralight()` |
| `font_light` | `.font_light()` |
| `font_normal` | `.font_normal()` |
| `font_medium` | `.font_medium()` |
| `font_semibold` | `.font_semibold()` |
| `font_bold` | `.font_bold()` |
| `font_extrabold` | `.font_extrabold()` |
| `font_black` | `.font_black()` |

## 布局快捷方法（已废弃）

> **⚠️ 已废弃**：`h_flex` / `v_flex` 已迁移到归一化样式属性。详见下方"归一化样式属性"章节。
>
> 旧写法 `h_flex=""` / `v_flex=""` 触发 deprecation warning 并被丢弃，请改用：
> ```xml
> <!-- 旧 -->
> <div h_flex>Horizontal Flex</div>
> <div v_flex>Vertical Flex</div>
>
> <!-- 新 -->
> <div display="flex" flex-direction="row">Horizontal Flex</div>
> <div display="flex" flex-direction="column">Vertical Flex</div>
> ```

## 归一化样式属性

RML 将 CSS 子集统一为声明式一等直接属性，避免散落的 Tailwind 式属性。归一化属性同时支持原生元素（`div`/`span` 等）与扩展组件（`Button`/`Card` 等，通过 `Styled` trait）。

### 语义快捷词

| 快捷词 | 等价值 | 生成代码 |
|--------|--------|----------|
| `width="full"` / `height="full"` | `width="100%"` / `height="100%"` | `.w_full()` / `.h_full()` |
| `min-width="0"` / `min-height="0"` | （CSS 0 关键字） | `.min_w_0()` / `.min_h_0()` |

### 归一化属性清单（对齐 `css/mapper.rs` 支持的 CSS 子集）

| 类别 | 属性 | 示例值 | 生成代码 |
|------|------|--------|----------|
| **盒模型** | `width` / `height` | `"100px"` / `"50%"` / `"full"` | `.w(gpui::px(100.))` / `.w(gpui::relative(0.5))` / `.w_full()` |
| | `padding` | `"10px"` / `"10px 20px"` | `.p(...)` / `.py(...).px(...)` |
| | `padding-top/right/bottom/left` | `"8px"` | `.pt(...)` 等 |
| | `margin` / `margin-top/right/bottom/left` | `"16px"` | `.m(...)` 等 |
| | `border-radius` | `"4px"` | `.rounded(...)` |
| | `border` / `border-color` | `"1px solid #ccc"` / `var(--border)` | `.border_1().border_color(...)` |
| | `border-top/right/bottom/left` | `"1px dashed ..."` | `.border_t_1()` 等 |
| **文本** | `font-size` | `"14px"` | `.text_size(...)` |
| | `font-weight` | `"bold"` / `"normal"` / `"medium"` | `.font_weight(FontWeight::BOLD)` |
| | `font-family` | `"Consolas"` | `.font_family("Consolas")` |
| | `text-align` | `"left"` / `"center"` / `"right"` | `.text_left()` 等 |
| | `line-height` | `"1.5"` 或 `"24px"` | `.line_height(...)` |
| | `white-space` | `"nowrap"` / `"pre"` / `"normal"` | `.whitespace_nowrap()` 等 |
| | `color` | `"red"` / `var(--text-color)` | `.text_color(gpui::rgb(...))` 或 `.text_color(rml::theme::color("--text-color"))` |
| | `background` / `background-color` | `"#f00"` / `var(--primary)` | `.bg(...)` |
| **Flexbox** | `display` | `"flex"` / `"none"` | `.flex()` / `.hidden()` |
| | `flex-direction` | `"row"` / `"column"` | `.flex_row()` / `.flex_col()` |
| | `flex-wrap` | `"wrap"` / `"nowrap"` | `.flex_wrap()` / `.flex_nowrap()` |
| | `justify-content` | `"center"` / `"flex-start"` / `"space-between"` | `.justify_center()` 等 |
| | `align-items` | `"center"` / `"stretch"` / `"flex-start"` | `.items_center()` 等 |
| | `flex` | `"1"` / `"2"` 等数字 | `.flex_grow(N).flex_shrink_0().flex_basis(gpui::px(0.))` |
| | `gap` | `"8px"` / `"16px"` | `.gap(gpui::px(8.))` |
| | `min-width` / `max-width` | `"0"` / `"50%"` | `.min_w_0()` / `.max_w(gpui::relative(0.5))` |
| | `min-height` / `max-height` | `"0"` / `"200px"` | `.min_h_0()` / `.max_h(...)` |
| **视觉效果** | `opacity` | `"0.5"` | `.opacity(0.5)` |
| | `overflow` / `overflow-x` / `overflow-y` | `"hidden"` / `"scroll"` / `"auto"` | `.overflow_hidden()` / `.overflow_x_scrollbar()` 等 |

### CSS 变量与运行时主题查询

`color` / `background` / `border-color` 等颜色属性支持 `var(--name)` 形式，生成运行时主题查询调用 `rml::theme::color("--name")`，主题切换即时生效。

```xml
<!-- 静态颜色：构建期内联 -->
<div color="red">红色文字</div>
<div background="#fff0f0">浅红背景</div>

<!-- CSS 变量：运行时主题查询（主题切换即时生效） -->
<div color="var(--text-color)">主题文字色</div>
<div background="var(--primary)">主题主色</div>
```

### 声明式语法

- 属性名按 kebab-case 书写（如 `flex-direction`、`padding-top`、`border-radius`）
- 静态形式：`width="100px"` / `display="flex"`
- **不支持 bind 形式**：`width={computed}` 输出 warning 并丢弃（运行时动态样式仍走 `class=` + 主题切换）

### 综合示例

```xml
<!-- 替代旧的 v-flex + gap-N -->
<div display="flex" flex-direction="column" gap="8px">
    <p>垂直布局，间距 8px</p>
    <p>第二行</p>
</div>

<!-- 替代旧的 h-flex + gap-N + items-center -->
<div display="flex" flex-direction="row" gap="16px" align-items="center">
    <Avatar name="Alice" />
    <div display="flex" flex-direction="column">
        <p>用户名</p>
        <p>角色</p>
    </div>
</div>

<!-- 替代旧的 h-full：CodeEditor 默认高度 360px，可用 height="full" 覆盖 -->
<CodeEditor height="full" value={code_sample} language="rml" />

<!-- 主题变量：颜色运行时查询 -->
<div background="var(--primary)" color="var(--text-on-primary)">主题按钮</div>
```

## 状态属性

```xml
<Button disabled>Disabled</Button>
<Button loading>Loading</Button>
<Button compact>Compact</Button>
<Button selected>Selected</Button>
```

| 属性 | 生成代码 |
|------|----------|
| `disabled` | `.disabled(true)` |
| `loading` | `.loading()` |
| `compact` | `.compact()` |
| `selected` | `.selected(true)` |

## 综合示例

```xml
<TabBar size="medium" underline selected-index={active_tab} on-click={on_tab_select}>
    <Tab label="General" />
    <Tab label="Advanced" />
</TabBar>

<DescriptionList size="small" vertical bordered columns={3} label-width="120" items={desitems} />

<Button size="large" primary on-click={on_save}>Save</Button>
```

## 规范要点

1. **size 用 medium 不用 middle**：所有组件统一使用 `medium`
2. **vertical 不重复 horizontal**：默认横向，仅 `vertical=true` 切换纵向
3. **variant 快捷方法省略值**：`primary` 等价于 `primary="true"`，但前者更简洁
4. **size 支持动态绑定**：`size={size_value}` 绑定到视图字段
5. **vertical 支持动态绑定**：`vertical={is_vertical}` 绑定到 bool 字段

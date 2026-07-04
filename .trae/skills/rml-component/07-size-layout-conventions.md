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

## 布局快捷方法

```xml
<div h_flex>Horizontal Flex</div>
<div v_flex>Vertical Flex</div>
```

| 属性 | 生成代码 |
|------|----------|
| `h_flex` | `.h_flex()` |
| `v_flex` | `.v_flex()` |

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

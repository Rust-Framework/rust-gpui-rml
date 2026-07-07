# TabBar

## 概述

`TabBar` 标签路由到 `rml_ui::TabBar`，是 **原生 gpui-component 形态的纯 header 标签栏**。仅承载标签行（header），无 body 切换、无关闭事件、无边框包裹。

`TabBar` 内部委托 [`Tabs`](./tabs.md) 渲染：当所有子项的 body 均为 `None` 时，`Tabs` 自动退化为 header-only 模式。`TabBar` 仅暴露 header 相关 API，不暴露 `bordered` / `on_close*` / `on_promote` 等 TabControl 专属方法。

RML **推荐使用 PascalCase** `<TabBar>`，与 gpui-component 原生 `TabBar` 类型名一致。kebab-case `<tab-bar>` 完全兼容。

## 标签别名表

| 写法 | 规范化结果 | 推荐度 | 说明 |
|------|-----------|--------|------|
| `<TabBar>` | `TabBar` | ✅ 推荐 | PascalCase，与 gpui-component 类型名一致 |
| `<tab-bar>` | `TabBar` | 兼容 | kebab-case，由 `normalize_component_tag` 处理 |
| `<Tab>` | `Tab` | ✅ 推荐 | 子项标签，统一底层为 `TabItem` |
| `<tab>` | `Tab` | 兼容 | 小写，`canonical_tag` 映射到 `Tab` |

> `<Tab>` 是 `TabBar` 与 `Tabs` 共用的子项标签。codegen 统一生成 `TabItem::new()...`，通过 `From<Tab> for TabItem` 转换（body=None）。详见 [tabs.md](./tabs.md)。

## 基本用法

最小示例 —— header-only 标签栏，点击切换选中：

```html
<TabBar selected-index={active_tab} on-click={on_tab_select}>
    <Tab label="Account" />
    <Tab label="Profile" />
    <Tab label="Settings" />
</TabBar>
```

- `selected-index={active_tab}` 绑定当前选中索引
- `on-click={on_tab_select}` 事件回调，签名 `fn(index: usize, &mut Window, &mut App)`
- `<Tab label="..." />` 子项，`label` 属性指定标题

## 5 种 variant

通过同名布尔标志切换 variant，所有子项继承：

```html
<TabBar underline="">      <!-- 下划线（iOS 风格） -->
<TabBar pill="">           <!-- 药丸（圆角背景） -->
<TabBar flat="">           <!-- 扁平（无边框，背景选中） -->
<TabBar outline="">        <!-- 描边（带边框） -->
<TabBar segmented="">      <!-- 分段（iOS SegmentedControl 风格） -->
```

默认 variant 为 `Tab`（标准标签栏，带 `tab_bar` 背景色）。

## 尺寸

通过 `size` 属性切换尺寸（Sizable 通用）：

```html
<TabBar size="xsmall">     <!-- 20px 高 -->
<TabBar size="small">      <!-- 24px 高 -->
<TabBar size="large">      <!-- 36px 高 -->
```

## 图标

`icon` 属性添加图标，值为 `rml_ui::IconName` 枚举名：

```html
<TabBar>
    <Tab icon="User" label="Account" />
    <Tab icon="Bell" label="Notifications" />
    <Tab icon="Settings" label="Settings" />
</TabBar>
```

`icon` 与 `label` 可组合使用（图标在左，文字在右）。优先级：children > icon > label。

## 禁用

`disabled` 属性禁用单个标签：

```html
<TabBar>
    <Tab label="Normal" />
    <Tab label="Disabled" disabled="true" />
</TabBar>
```

## menu 模式

`menu` 属性启用下拉菜单（标签过多时显示"更多"按钮）：

```html
<TabBar menu="true">
    <Tab label="Tab 1" />
    <Tab label="Tab 2" />
    <Tab label="Tab 3" />
    <Tab label="Tab 4" />
    <Tab label="Tab 5" />
    <Tab label="Tab 6" />
</TabBar>
```

## prefix / suffix

`prefix` / `suffix` 在标签行首尾注入元素：

```html
<TabBar prefix={<Icon name="Sidebar" />} suffix={<Button label="+" />}>
    <Tab label="Tab 1" />
    <Tab label="Tab 2" />
</TabBar>
```

## header 自定义插槽

当 `label` + `icon` 不足以表达标题时，用 `<template slot="header">` 注入任意元素：

```html
<TabBar selected-index={active_tab} on-click={on_tab_select}>
    <Tab>
        <template slot="header">
            <span>Account</span>
            <Badge>3</Badge>
        </template>
    </Tab>
    <Tab>
        <template slot="header">
            <span>Profile</span>
        </template>
    </Tab>
</TabBar>
```

## closable（仅视觉）

`<Tab closable>` 会在 header 末尾渲染关闭按钮（`IconName::Close`，`.xsmall()` 尺寸），按钮**仅在鼠标 hover 到该 tab 时显示**。

> **注意**：`TabBar` 不暴露 `on_close` 事件。关闭按钮点击会触发 `stop_propagation()` 并冒泡到 `on_click`，但 `TabBar` 不提供关闭回调。若需关闭事件，请使用 [`<Tabs>`](./tabs.md) 的 `on-close` 属性。

## TabBar 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `selected-index` | `usize` | `{expr}` | 当前选中索引 |
| `on-click` | 事件 | `="method"` | 点击回调，签名 `fn(index: usize, &mut Window, &mut App)` |
| `underline` / `pill` / `flat` / `outline` / `segmented` | 布尔标志 | — | 5 种 variant |
| `size` | `xsmall` / `small` / `large` | — | 尺寸 |
| `menu` | 布尔 | — | 启用下拉菜单（标签过多时） |
| `prefix` / `suffix` | 元素 | `{expr}` | 首尾注入元素 |
| `last-empty-space` | 元素 | `{expr}` | 尾部占位元素 |
| `track-scroll` | `ScrollHandle` 引用 | `{expr}` | 滚动控制 |

> **不支持的属性**（Tabs 专属，TabBar 未暴露）：`bordered` / `on-close` / `on-close-all` / `on-close-others` / `on-promote`。

## Tab 子项属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 标签标题 |
| `icon` | `IconName::...` | `{expr}` | 标签图标（与 `label` 互斥，优先级：children > icon > label） |
| `disabled` | 布尔 | — | 禁用标签 |
| `selected` | 布尔 | — | 选中状态 |
| `closable` | 布尔 | — | 是否渲染关闭按钮（hover 时显示，仅视觉，TabBar 无 on_close 事件） |
| `on-click` | 事件 | `="method"` | 单个标签点击回调（ClickEvent） |
| `<template slot="header">` | 插槽 | — | header 自定义内容（覆盖 label/icon） |
| 子节点 | element | — | header 自定义内容（等同 header 插槽） |

> **Tab 子节点在 TabBar 中仅作 header 渲染**。若需 body 内容面板（WPF TabControl 模式），请使用 [`<Tabs>`](./tabs.md)。

## 与 Tabs 的选择指南

| 场景 | 推荐组件 |
|------|----------|
| 纯 header 标签切换，无 body | `<TabBar>` |
| 标签栏 + 内容面板一体化（WPF TabControl） | `<Tabs>` |
| 需要关闭按钮 + on_close 事件 | `<Tabs>` |
| 需要 bordered 边框包裹整体 | `<Tabs>` |
| tab_window 标题栏内的标签栏 | `<TabBar>`（header-only）+ 外部 `<component>` 注入 body |

## 相关文档

- [tabs.md](./tabs.md) — WPF TabControl 风格（header + body）
- [window-roots.md](./window-roots.md) — tab_window 根节点与插槽分区
- [slots.md](../slots.md) — `<template slot="...">` 插槽机制

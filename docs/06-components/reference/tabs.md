# Tabs

## 概述

`Tabs` 标签路由到 `rml_ui::Tabs`，是 **WPF TabControl 风格的标签容器**：header（标签行）+ body（内容面板）一体化切换。对应 WPF `<TabControl><TabItem Header="...">Content</TabItem></TabControl>` 模式。

与 [`TabBar`](./tab-bar.md) 的关键差异：

| 特性 | `Tabs` | `TabBar` |
|------|--------|----------|
| header 标签行 | ✅ | ✅ |
| body 内容面板 | ✅ | ❌ |
| `bordered` 边框包裹整体 | ✅ | ❌ |
| `on-close` / `on-close-all` / `on-close-others` | ✅ | ❌ |
| `on-promote`（双击 promote） | ✅ | ❌ |
| 右键菜单（Close/Close All/Close Others） | ✅ | ❌ |

`TabBar` 是 `Tabs` 的 header-only 子集，内部委托 `Tabs` 渲染。当所有子项 body 均为 `None` 时，`Tabs` 自动退化为 header-only 模式。

RML **推荐使用 PascalCase** `<Tabs>`，与 gpui-component `Tabs` 类型名一致。kebab-case `<tabs>` 完全兼容。

## 标签别名表

| 写法 | 规范化结果 | 推荐度 | 说明 |
|------|-----------|--------|------|
| `<Tabs>` | `Tabs` | ✅ 推荐 | PascalCase，与 gpui-component 类型名一致 |
| `<tabs>` | `Tabs` | 兼容 | 小写，`canonical_tag` 映射到 `Tabs` |
| `<Tab>` | `Tab` | ✅ 推荐 | 子项标签，统一底层为 `TabItem` |
| `<tab>` | `Tab` | 兼容 | 小写 |

> `<Tab>` 是 `Tabs` 与 `TabBar` 共用的子项标签。codegen 统一生成 `TabItem::new()...`，在 `<Tabs>` 内可带 body 子节点，在 `<TabBar>` 内仅作 header 渲染。

## 基本用法（body 模式）

`<Tab>` 的子节点作为 body 内容（对应 WPF `TabItem.Content`），仅选中 tab 的 body 被渲染：

```html
<Tabs selected-index={active_tab} on-click={on_tab_select}>
    <Tab label="Account">
        <div class="tab-body">
            <p>Account settings panel</p>
        </div>
    </Tab>
    <Tab label="Profile">
        <div class="tab-body">
            <p>User profile panel</p>
        </div>
    </Tab>
    <Tab label="Settings">
        <div class="tab-body">
            <p>System settings panel</p>
        </div>
    </Tab>
</Tabs>
```

- `selected-index={active_tab}` 绑定当前选中索引
- `on-click={on_tab_select}` 事件回调，签名 `fn(index: usize, &mut Window, &mut App)`
- `<Tab label="...">` 的 `label` 属性对应 WPF `TabItem.Header`
- `<Tab>` 的子节点（`<div class="tab-body">`）对应 WPF `TabItem.Content`，选中时惰性渲染

## bordered 属性

`bordered` 启用 1px 边框，**包裹 header + body 整体**（而非仅 header）：

```html
<Tabs bordered="" selected-index={active_tab} on-click={on_tab_select}>
    <Tab label="Tab 1">
        <p>Content 1</p>
    </Tab>
    <Tab label="Tab 2">
        <p>Content 2</p>
    </Tab>
</Tabs>
```

运行时渲染结构（[tabs.rs:820-829](../../../crates/ui/src/components/tab/tabs.rs)）：

```text
v_flex (size_full, border_1, border_color)
├── header（Tabs 横向标签行）
└── div (flex_1, min_h_0)
    └── body（选中 tab 的 body 闭包渲染结果）
```

> `bordered` 是 `Tabs` 专属属性，`TabBar` 不支持。

## 5 种 variant

与 `TabBar` 一致，通过同名布尔标志切换：

```html
<Tabs underline="">      <!-- 下划线（iOS 风格） -->
<Tabs pill="">           <!-- 药丸（圆角背景） -->
<Tabs flat="">           <!-- 扁平（无边框，背景选中） -->
<Tabs outline="">        <!-- 描边（带边框） -->
<Tabs segmented="">      <!-- 分段（iOS SegmentedControl 风格） -->
```

默认 variant 为 `Tab`（标准标签栏）。

## 关闭按钮与事件

`<Tab closable>` 在 header 末尾渲染关闭按钮（`IconName::Close`，`.xsmall()` 尺寸），按钮**仅在鼠标 hover 到该 tab 时显示**。关闭按钮点击触发 `stop_propagation()`，避免误触发 tab 的 `on_click`。

`Tabs` 容器级关闭事件：

| 属性 | 签名 | 说明 |
|------|------|------|
| `on-close` | `fn(index: usize, &mut Window, &mut App)` | 关闭按钮点击回调，参数为被关闭 tab 的索引 |
| `on-close-all` | `fn(&mut Window, &mut App)` | "关闭全部"回调（无索引参数） |
| `on-close-others` | `fn(index: usize, &mut Window, &mut App)` | "关闭其他"回调，参数为保留 tab 的索引 |

```html
<Tabs
    selected-index={selected_index}
    on-click={on_tab_select}
    on-close={on_tab_close}
    on-close-all={on_tab_close_all}
    on-close-others={on_tab_close_others}>
    <Tab each={tab in tabs}
        label={tab.title}
        closable={tab.closable} />
</Tabs>
```

## 右键菜单（内置）

当 `on-close` / `on-close-all` / `on-close-others` 任一存在时，`Tabs` 自动为每个 tab 注入右键菜单，包含框架内置三个标准菜单项：

| 菜单项 | i18n key | 触发条件 |
|--------|----------|----------|
| Close | `rml.tab.close` | `on-close` 存在 |
| Close Others | `rml.tab.close_others` | `on-close-others` 存在 |
| Close All | `rml.tab.close_all` | `on-close-all` 存在 |

菜单项文本走 i18n（`rml_core::i18n::t_or_default`），可通过 i18n 资源覆盖。

## 预览模式与 promote

`<Tab preview>` 以 italic 标题渲染（VSCode 预览 tab 风格）。双击 tab 触发 `on-promote` 回调（250ms 时间窗口内两次点击视为双击）。

| 属性 | 签名 | 说明 |
|------|------|------|
| `on-promote` | `fn(index: usize, &mut Window, &mut App)` | 双击 tab 触发，参数为被双击 tab 的索引 |

```html
<Tabs selected-index={selected_index}
    on-click={on_tab_select}
    on-promote={on_tab_promote}>
    <Tab each={tab in tabs}
        label={tab.title}
        preview={tab.preview} />
</Tabs>
```

> **VSCode 预览 tab 模式**：单击打开为 preview（italic 标题），双击 promote 为持久 tab。业务通过 `preview` 标志 + `on-promote` 回调驱动数据层变更。

## 溢出压缩（自适应紧凑模式）

当 `menu="true"` 且 Tab 数量超出可视宽度时，`Tabs` 自动从**滚动模式**切换为**压缩模式**（类似浏览器 Tab 行为）：

| 模式 | 触发条件 | Tab 行为 | 容器行为 |
|------|----------|----------|----------|
| 滚动模式 | 内容宽度 ≤ 视口宽度 | `flex_shrink_0`（固定宽度）+ 完整 label | `overflow_x_scroll`，可横向滚动 |
| 压缩模式 | 内容宽度 > 视口宽度 | `flex_1` + `min_w_0` + label `truncate()`（省略号） | `overflow_x_hidden`，等分容器宽度 |

**实现要点**（[tabs.rs:494-657](../../../crates/ui/src/components/tab/tabs.rs)）：

1. `Tabs` 在 `on_prepaint` 阶段测量 `content_width` 与 `viewport_width`，结果存入 `Entity<bool>` 溢出标志
2. 溢出时把 `compress=true` 传播给每个 `Tab`，并切换容器为 `overflow_x_hidden`
3. `Tab` 收到 `compress=true` 后：自身 `flex_1().min_w_0()`，label 包裹 `div().min_w_0().truncate()` 实现等分 + 省略号
4. 首帧无 bounds（测量延迟一帧），第二帧拾取后自动 re-render

> **设计意图**：浏览器式"会呼吸"的 Tab 行——少时宽松、多时紧凑、关闭后自动恢复。无需业务侧配置。

## Tabs 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `selected-index` | `usize` | `{expr}` | 当前选中索引 |
| `on-click` | 事件 | `="method"` | 点击回调，签名 `fn(index: usize, &mut Window, &mut App)` |
| `on-close` | 事件 | `="method"` | 关闭按钮回调，签名 `fn(index: usize, &mut Window, &mut App)` |
| `on-close-all` | 事件 | `="method"` | 关闭全部回调，签名 `fn(&mut Window, &mut App)` |
| `on-close-others` | 事件 | `="method"` | 关闭其他回调，签名 `fn(index: usize, &mut Window, &mut App)` |
| `on-promote` | 事件 | `="method"` | 双击 promote 回调，签名 `fn(index: usize, &mut Window, &mut App)` |
| `bordered` | 布尔标志 | — | 1px 边框包裹 header + body 整体 |
| `underline` / `pill` / `flat` / `outline` / `segmented` | 布尔标志 | — | 5 种 variant |
| `size` | `xsmall` / `small` / `large` | — | 尺寸 |
| `menu` | 布尔 | — | 启用下拉菜单 + 溢出压缩（标签过多时） |
| `prefix` / `suffix` | 元素 | `{expr}` | 首尾注入元素 |
| `last-empty-space` | 元素 | `{expr}` | 尾部占位元素 |
| `track-scroll` | `ScrollHandle` 引用 | `{expr}` | 滚动控制 |

## Tab 子项属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 标签标题（对应 WPF `TabItem.Header`） |
| `icon` | `IconName::...` | `{expr}` | 标签图标（与 `label` 互斥，优先级：children > icon > label） |
| `disabled` | 布尔 | — | 禁用标签 |
| `selected` | 布尔 | — | 选中状态 |
| `closable` | 布尔 | — | 是否渲染关闭按钮（hover 时显示，触发 `on-close`） |
| `preview` | 布尔 | — | 预览模式（italic 标题，VSCode 风格） |
| `on-click` | 事件 | `="method"` | 单个标签点击回调（ClickEvent） |
| `<template slot="header">` | 插槽 | — | header 自定义内容（覆盖 label/icon） |
| 子节点 | element | — | body 内容（对应 WPF `TabItem.Content`，选中时渲染） |

> **body 与 header 插槽的区别**：直接子节点作为 body 渲染（选中时显示）；`<template slot="header">` 内的子节点作为 header 自定义内容（始终显示在标签行内）。

## 运行时渲染结构

### body 模式（有子节点）

```text
v_flex (size_full)
├── [border_1, border_color]    ← 仅 bordered=true 时
├── header（Tabs 横向标签行）
│   ├── prefix
│   ├── Tab[0] ... Tab[N]
│   ├── last_empty_space
│   └── suffix
│       └── [menu button]       ← 仅 menu=true 且溢出时
└── div (flex_1, min_h_0)
    └── body（选中 tab 的 body 闭包渲染结果）
```

### header-only 模式（无子节点）

当所有 `<Tab>` 均无 body 子节点时，`Tabs` 退化为仅渲染 header（与 `TabBar` 等效）。

## 完整示例

### WPF TabControl 模式 + bordered

```html
<Tabs bordered="" selected-index={active_tab} on-click={on_tab_select}>
    <Tab label="Account">
        <div class="tab-body">
            <p>Account settings panel</p>
        </div>
    </Tab>
    <Tab label="Profile">
        <div class="tab-body">
            <p>User profile panel</p>
        </div>
    </Tab>
    <Tab label="Settings">
        <div class="tab-body">
            <p>System settings panel</p>
        </div>
    </Tab>
</Tabs>
```

### VSCode 风格（关闭 + promote + 溢出压缩）

```html
<Tabs
    selected-index={selected_index}
    on-click={on_tab_select}
    on-close={on_tab_close}
    on-close-all={on_tab_close_all}
    on-close-others={on_tab_close_others}
    on-promote={on_tab_promote}
    menu="true">
    <Tab each={tab in tabs}
        label={tab.title}
        closable={tab.closable}
        preview={tab.preview} />
</Tabs>
```

### 代码示例切换（.rml / .rml.rs）

```html
<Tabs selected-index={code_tab} on-click={on_code_tab_change}>
    <Tab label=".rml">
        <CodeEditor ref="rml_editor" value={rml_sample} language="rml" />
    </Tab>
    <Tab label=".rml.rs">
        <CodeEditor ref="rust_editor" value={rust_sample} language="rust" />
    </Tab>
</Tabs>
```

## 与 TabBar 的选择指南

| 场景 | 推荐组件 |
|------|----------|
| 标签栏 + 内容面板一体化（WPF TabControl） | `<Tabs>` |
| 需要关闭按钮 + on_close 事件 | `<Tabs>` |
| 需要 bordered 边框包裹整体 | `<Tabs>` |
| 需要 promote（双击） | `<Tabs>` |
| 纯 header 标签切换，无 body | `<TabBar>` |
| tab_window 标题栏内的标签栏 | `<TabBar>`（header-only）+ 外部 `<component>` 注入 body |

## 相关文档

- [tab-bar.md](./tab-bar.md) — 原生 header-only 标签栏
- [window-roots.md](./window-roots.md) — tab_window 根节点与插槽分区
- [slots.md](../slots.md) — `<template slot="...">` 插槽机制
- [composition.md](../composition.md) — 组件组合模式

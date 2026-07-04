# TabBar / Tab / tab-item

## 概述

RML 标签栏由三个标签组成，对应两种使用模式：

| 标签 | 承载内容 | 适用场景 |
|------|----------|----------|
| `<TabBar>` | 容器 | 标签栏本体，承载子项并管理选中态 |
| `<Tab>` | **仅 header**（label / icon / children） | tab_window 标题栏内、或独立 TabBar 仅需 header 时 |
| `<tab-item>` | **header + body**（title + 子节点） | 独立 TabBar 需要 WPF TabControl 模式（header + 切换 body）时 |

> **关键差异**：`<Tab>` 是 header-only 子集；`<tab-item>` 是 header + body 超集。运行时 [`TabItem::into_header_tab()`](../../../crates/ui/src/components/tab/tab_item.rs) 把 title 部分转换为 `Tab` 做 header 渲染，body 部分作为闭包惰性渲染（仅选中 tab 的 body 被调用）。

## 两种使用模式

### 模式一：Header-only（`<Tab>`）

`<Tab>` 仅提供 header 内容，无 body 概念。子节点作为 header 自定义内容（绕过 label/icon 限制）。

```html
<TabBar selected-index={active_tab} on-click={on_tab_select}>
    <Tab label="Account" />
    <Tab icon="User" label="Profile" />
    <Tab>
        <span>Settings</span>
        <Badge>3</Badge>
    </Tab>
</TabBar>
```

**典型场景**：tab_window 标题栏内的 TabBar。标题栏高度仅 32px，body 放不下，所以 body 由外部 `<component content={active_view}>` 单独注入到主体区。

### 模式二：WPF TabControl（`<tab-item>`）

`<tab-item>` 同时承载 title（header）与 body（子节点）。`title` 属性对应 WPF `TabItem.Header`，子节点对应 `TabItem.Content`。

```html
<TabBar selected-index={active_tab} on-click={on_tab_select}>
    <tab-item title="Account">
        <div class="tab-body">
            <p>Account settings panel</p>
        </div>
    </tab-item>
    <tab-item title="Profile">
        <div class="tab-body">
            <p>User profile panel</p>
        </div>
    </tab-item>
</TabBar>
```

**运行时渲染**（[tab_bar.rs:634-642](../../../crates/ui/src/components/tab/tab_bar.rs)）：

```text
v_flex (size_full)
├── header（TabBar 横向标签行）
└── div (flex_1, min_h_0)
    └── body（选中 tab 的 body 闭包渲染结果）
```

**典型场景**：独立 `<TabBar>` 放在主体区（非标题栏），需要 header + body 一体化切换时。对应 WPF `<TabControl><TabItem Header="...">Content</TabItem></TabControl>` 模式。

## 两种模式的关系

```text
┌─────────────────────────────────────────────────────────────────┐
│ <tab-item>                                                      │
│  ├── title 部分 ──→ into_header_tab() ──→ <Tab> (header 渲染)   │
│  └── body 部分  ──→ 闭包 ──→ 选中时渲染                          │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ From<Tab> for TabItem
                              │ (body=None)
┌─────────────────────────────────────────────────────────────────┐
│ <Tab>                                                           │
│  └── header 内容（label / icon / children）                     │
└─────────────────────────────────────────────────────────────────┘
```

- `<Tab>` 是 header-only 子集；`<tab-item>` 是 header + body 超集
- `TabItem::into_header_tab()` 把 TabItem 的 title 部分转换为 Tab 做 header 渲染（保留 6 种 variant 动画/状态）
- `From<Tab> for TabItem` 允许现有 `TabBar::child(Tab::new()...)` 调用方式仍然有效（body=None）
- 当 TabBar 检测到任意子项有 body 时，整体渲染切换为 `v_flex > [header, body]` 布局

## 为何 main_window.rml 用 `<Tab>` 而非 `<tab-item>`

[`demo/src/shell/main_window.rml`](../../../demo/src/shell/main_window.rml) 的 TabBar 被 tab_window shell 嵌在 32px 高的标题栏内（[tab_window.rs:521-562](../../../crates/ui/src/window/tab_window.rs)）：

```text
| 图标切换 | 主窗口菜单 | Title | Tab1 | Tab2 | … | 可扩展区 | 窗口操作 |
└──────────────────── 32px 标题栏 ────────────────────────────────┘
```

即便 `<tab-item>` 有 body，也会被压在 32px 标题栏高度内无法显示。所以 main_window 采用**拆分方案**：

```html
<tab_window ... on-tab-click="on_tab_click" on-tab-close="on_tab_close">
    <!-- 1. 标题栏内：仅 header（<Tab>） -->
    <template slot="tabs" each={w in workbenches}>
        <Tab label={w.name()} closable />
    </template>

    <!-- 2. 主体区：外部 <component> 注入 body -->
    <component content={self.active_view(_window, cx)} />
</tab_window>
```

- **标题栏**：`<Tab>` 仅提供 header，body 由 `<component content={active_view}>` 单独注入到主体区
- **body 来源**：`active_view()` 读 `activated` workbench 调用 `IVisualContribution::render`

这是**布局约束下的合理拆分**，不是设计冗余。当 TabBar 不在标题栏（如 tab_bar_case.rml 的独立 TabBar demo）时，可直接用 `<tab-item>` 一体化 header + body。

## TabBar 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `selected-index` | `usize` | `{expr}` | 当前选中索引 |
| `on-click` | 事件 | `="method"` | 点击回调，签名 `fn(index: usize, &mut Window, &mut App)` |
| `underline` / `pill` / `flat` / `outline` / `segmented` | 布尔标志 | — | 5 种 variant |
| `size` | `xsmall` / `small` / `large` | — | 尺寸 |
| `menu` | 布尔 | — | 启用下拉菜单（标签过多时） |
| `prefix` / `suffix` | 元素 | `{expr}` | 首尾注入元素 |
| `on-close` | 事件 | — | Tab 关闭回调（TabBar 级），签名同 `on-click` |

## Tab 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 标签标题 |
| `icon` | `IconName::...` | `{expr}` | 标签图标（与 `label` 互斥，优先级：children > icon > label） |
| `disabled` | 布尔 | — | 禁用标签 |
| `selected` | 布尔 | — | 选中状态 |
| `closable` | 布尔 | — | 是否渲染关闭按钮（hover 时显示） |
| 子节点 | element | — | header 自定义内容（绕过 label/icon 限制，按 `.child()` 顺序注入） |

## tab-item 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `title` | 字符串 | `{expr}` | header 标题（对应 WPF `TabItem.Header`） |
| `closable` | 布尔 | — | 透传到 `Tab::closable` |
| 子节点 | element | — | body 内容（对应 WPF `TabItem.Content`，选中时渲染） |

## closable 与 on-tab-close

`<Tab closable>` 与 `<tab-item closable>` 都会在 header 末尾渲染关闭按钮（`IconName::Close`，`.xsmall()` 尺寸），按钮**仅在鼠标 hover 到该 tab 时显示**（gpui-component `group`/`group_hover` 机制）。

关闭按钮点击会触发 `stop_propagation()`，避免误触发 tab 的 `on_click`。事件向上冒泡到 TabBar 的 `on_close` 回调，再到 tab_window shell 的 `on_tab_close` 事件。

```html
<tab_window ... on-tab-click="on_tab_click" on-tab-close="on_tab_close">
    <template slot="tabs" each={w in workbenches}>
        <Tab label={w.name()} closable />
    </template>
    ...
</tab_window>
```

```rust
// main_window.rml.rs
#[command]
pub fn on_tab_close(&mut self, index: usize, cx: &mut Context<Self>) {
    let wb = self.workbenches.snapshot().get(index).cloned();
    if let Some(wb) = wb {
        let uri: Uri = wb.uri().parse().unwrap();
        IWorkbenchManager::close(self, &uri);
        self.__rml_bump_version("activated");
        cx.notify();
    }
}
```

`IWorkbenchManager::close` 内部 `remove_where` 移除 workbench，并按**就近左侧激活**策略重定向 `activated`：关闭索引 N → 激活 N-1；关闭首项 → 激活新首项；无剩余 → `None`。匹配浏览器 Tab 关闭交互。

## 溢出压缩（自适应紧凑模式）

当 Tab 数量超出 TabBar 可视宽度时，TabBar 自动从**滚动模式**切换为**压缩模式**（类似浏览器 Tab 行为）：

| 模式 | 触发条件 | Tab 行为 | 容器行为 |
|------|----------|----------|----------|
| 滚动模式 | 内容宽度 ≤ 视口宽度 | `flex_shrink_0`（固定宽度）+ 完整 label | `overflow_x_scroll`，可横向滚动 |
| 压缩模式 | 内容宽度 > 视口宽度 | `flex_1` + `min_w_0` + label `truncate()`（省略号） | `overflow_x_hidden`，等分容器宽度 |

**实现要点**（[tab_bar.rs:494-497](../../../crates/ui/src/components/tab/tab_bar.rs), [tab.rs:466,626-632,824](../../../crates/ui/src/components/tab/tab.rs)）：

1. TabBar 在 `on_prepaint` 阶段测量 `content_width` 与 `viewport_width`，结果存入 `Entity<bool>` 溢出标志
2. 溢出时把 `compress=true` 传播给每个 Tab，并切换容器为 `overflow_x_hidden`
3. Tab 收到 `compress=true` 后：自身 `flex_1().min_w_0()`，label 包裹 `div().min_w_0().truncate()` 实现等分 + 省略号
4. 首帧无 bounds（测量延迟一帧），第二帧拾取后自动 re-render

> **设计意图**：浏览器式"会呼吸"的 Tab 行——少时宽松、多时紧凑、关闭后自动恢复。无需业务侧配置。

## 关闭后激活策略

关闭一个 Tab 后，激活项遵循**就近左侧激活**原则（[main_window.rml.rs:463-484](../../../demo/src/shell/main_window.rml.rs)）：

| 关闭索引 N | 新激活项 |
|------------|----------|
| N > 0 | N-1（左邻） |
| N = 0（首项） | 新首项（原 index 1） |
| 无剩余项 | `None` |

实现使用 `saturating_sub(1)` 防止下溢：`new_snapshot.get(ix.saturating_sub(1)).cloned()`。

## 完整示例

### 独立 TabBar + WPF TabControl 模式

参见 [`demo/src/cases/tab_bar_case.rml`](../../../demo/src/cases/tab_bar_case.rml)：

```html
<TabBar selected-index={active_tab} on-click={on_tab_select}>
    <tab-item title="Account">
        <div class="tab-body">
            <p>Account settings panel</p>
        </div>
    </tab-item>
    <tab-item title="Profile">
        <div class="tab-body">
            <p>User profile panel</p>
        </div>
    </tab-item>
    <tab-item title="Settings">
        <div class="tab-body">
            <p>System settings panel</p>
        </div>
    </tab-item>
</TabBar>
```

### tab_window 标题栏 + 外部 body 注入

参见 [`demo/src/shell/main_window.rml`](../../../demo/src/shell/main_window.rml)：

```html
<tab_window ... tabs={tab_bar_items} selected_tab={selected_tab}
            on-tab-click="on_tab_click" on-tab-close="on_tab_close">

    <!-- 标题栏内：仅 header -->
    <template slot="tabs" each={w in workbenches}>
        <Tab label={w.name()} closable />
    </template>

    <!-- 主体区：外部注入 body -->
    <component content={self.active_view(_window, cx)} />

    <template slot="left">
        <ActivityBar panels={activity_panels} on_panel_change="on_panel_change" />
    </template>
</tab_window>
```

## 选择指南

| 场景 | 推荐方案 |
|------|----------|
| tab_window 标题栏内的 TabBar | `<Tab>`（header-only）+ 外部 `<component>` 注入 body |
| 独立 TabBar，需要 header + body 一体化切换 | `<tab-item>`（WPF TabControl 模式） |
| 独立 TabBar，仅 header 切换 | `<Tab>`（header-only） |
| 需要 tab 关闭功能 | 任一方案 + `closable` 属性 + `on-close` / `on-tab-close` 事件 |

## 相关文档

- [window-roots.md](./window-roots.md) — tab_window 根节点与插槽分区
- [slots.md](../slots.md) — `<template slot="...">` 插槽机制
- [composition.md](../composition.md) — 组件组合模式

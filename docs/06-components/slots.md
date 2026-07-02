# 6.3 插槽与内容分发

> **本节目标**：掌握 `<template slot="...">` 的用法，实现 shell 窗口的内容分发，提升窗口布局的灵活性。

## 当前实现状态

RML 框架当前已实现 **shell 级插槽**（`<tab_window>` / `<modern_window>` 根节点）：

- ✅ **具名插槽填充**：`<template slot="name">...</template>` 形式，已可用于 tab_window / modern_window
- ✅ **插槽契约声明**：`#[component(slots = ["header", "footer"])]` 宏参数（基础设施已就绪）
- 🚧 **自定义组件插槽分发**：用户自定义 `#[component]` 组件的 `<slot>` 占位符渲染（规划中）
- 🚧 **作用域插槽** `<slot let-item={item}>`：与 `each` 列表渲染配合（规划中）

## 6.3.1 插槽的概念

插槽（Slot）是内容分发的机制，允许父视图向窗口外壳（shell）传递任意内容到指定位置：

```
父视图 (.rml)             Shell 组件 (TabWindowShell)
┌──────────────────┐      ┌──────────────────────────┐
│ <tab_window>     │      │  ┌────────────────────┐  │
│   <template      │      │  │ TitleBar           │  │
│    slot="menu">  │──传递──▶│   [menu slot 占位] │  │
│     ...          │      │  ├────────────────────┤  │
│   </template>    │      │  │ Body               │  │
│   ...            │      │  ├────────────────────┤  │
│ </tab_window>    │      │  │ [footer slot 占位] │  │
└──────────────────┘      │  └────────────────────┘  │
                          └──────────────────────────┘
```

## 6.3.2 Shell 窗口插槽（已实现）

### TabWindowShell 支持的插槽

| 插槽名 | 位置 | 用途 |
|--------|------|------|
| `menu` | 标题栏左侧 | 主菜单栏 |
| `title` | 标题栏中部 | 标题扩展区（按钮等） |
| `footer` | 底部 | 状态栏 |
| `left` | 主体左侧 | 左侧面板（可 resize） |
| `right` | 主体右侧 | 右侧面板（可 resize） |
| `bottom` | 底部主体 | 输出面板（可 resize） |

### 使用示例

```html
<tab_window title="My App" tabs={tab_items} selected_tab={selected}>
    <template slot="left">
        <ActivityBar ref="activity_bar" />
    </template>

    <template slot="menu">
        <menu-bar items={menu_items} />
    </template>

    <template slot="title">
        <Button label="Docs" ghost="" />
    </template>

    <template slot="bottom">
        <div>Output panel — drag the top edge to resize</div>
    </template>

    <template slot="footer">
        <status_bar items={status_items} />
    </template>

    <!-- 主内容（无 slot 属性的子节点） -->
    <component content={self.active_view(_window, cx)} />
</tab_window>
```

### ModernWindowShell 支持的插槽

| 插槽名 | 位置 | 用途 |
|--------|------|------|
| `menu` | 标题栏左侧 | 主菜单栏 |
| `title` | 标题栏中部 | 标题扩展区 |
| `footer` | 底部 | 状态栏 |

## 6.3.3 插槽契约声明（`#[component]` 宏）

组件可通过 `#[component(slots = [...])]` 宏参数显式声明它接受的插槽列表：

```rust
#[component(slots = ["header", "footer", "default"])]
pub struct Card { ... }
```

- `slots` 参数为字符串数组字面量
- 保留名 `"default"` 对应默认插槽
- 不写 `slots` 参数 → 组件不接受任何插槽

> 注：当前 shell 组件（TabWindowShell / ModernWindowShell）已内置插槽支持，无需手动声明。
> 自定义组件的插槽内容分发机制正在规划中。

## 6.3.4 规划中的特性

以下特性尚未实现，文档作为设计预览保留：

### 默认插槽与 `<slot>` 占位符（规划中）

组件模板内用 `<slot>` 定义内容占位符：

```html
<!-- components/card.rml（规划中） -->
<div class="card">
    <div class="card-body">
        <slot />
    </div>
</div>
```

### 作用域插槽（规划中）

`<slot let-item={item}>` 向父视图暴露数据，与 `each` 列表渲染配合：

```html
<!-- components/list.rml（规划中） -->
<ul>
    <li each={item in items}>
        <slot let-item={item} let-index={index}></slot>
    </li>
</ul>
```

```html
<!-- 父视图（规划中） -->
<List items={my_items}>
    <template let-item let-index>
        <span>{index}: {item.name}</span>
    </template>
</List>
```

## 6.3.5 小结

RML 插槽当前支持：

- **Shell 窗口插槽**：`<template slot="name">` 填充 tab_window / modern_window 的具名位置
- **插槽契约声明**：`#[component(slots = [...])]` 宏参数（基础设施）
- **6 个 shell 插槽**：menu / title / footer / left / right / bottom

规划中：

- 自定义组件 `<slot>` 占位符渲染
- 默认插槽内容
- 作用域插槽 `<slot let-item={item}>`

掌握 shell 插槽，你就能灵活定制窗口布局。

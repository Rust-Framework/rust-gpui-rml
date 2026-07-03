# 6.3 插槽与内容分发

> **本节目标**：掌握 `<slot>` 占位符与 `<template slot="...">` 填充的标准语法，实现 shell 窗口与自定义组件的内容分发。

## 当前实现状态

RML 框架已实现 **Vue 风格插槽**，覆盖 shell 窗口与自定义组件两个层级：

- ✅ **Shell 窗口插槽**：`<tab_window>` / `<modern_window>` 根节点的 `<template slot="name">` 填充
- ✅ **自定义组件插槽**：`#[component(slots = [...])]` 契约 + 模板内 `<slot>` 占位符 + 父视图 `<template slot="...">` 填充
- ✅ **编译期校验**：未知 slot 名 / 未知属性报 error，已注册未映射属性报 warning
- 🚧 **作用域插槽** `<slot let-item={item}>`：与 `each` 列表渲染配合（规划中）

## 6.3.1 插槽的概念

插槽（Slot）是内容分发机制，允许父视图向子组件的指定位置传递任意内容：

```
父视图 (.rml)               子组件模板 (.rml)
┌──────────────────┐        ┌──────────────────────────┐
│ <Card>           │        │  ┌────────────────────┐  │
│   <template      │        │  │ [header slot 占位] │  │
│    slot="header">│──传递──▶│  ├────────────────────┤  │
│     ...          │        │  │ [default slot 占位]│  │
│   </template>    │        │  ├────────────────────┤  │
│   <p>正文</p>    │──传递──▶│  │ [footer slot 占位] │  │
│   <template      │        │  └────────────────────┘  │
│    slot="footer">│──传递──▶│                          │
│     ...          │        │                          │
│   </template>    │        │                          │
│ </Card>          │        │                          │
└──────────────────┘        └──────────────────────────┘
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

> 注：shell 组件已内置插槽支持，无需手动声明 `#[component(slots)]`。

## 6.3.3 自定义组件插槽（已实现）

自定义组件通过**两处声明**预留插槽，父视图通过 `<template slot="...">` 填充。

### ① 组件开发者：声明插槽契约

**Rust 侧**用 `#[component(slots = [...])]` 宏参数声明组件接受的插槽列表：

```rust
// components/card.rml.rs
use rml::prelude::*;

#[derive(IModel)]
#[component(slots = ["header", "default", "footer"])]
pub struct Card {
    pub title: SharedString,
}

impl Card {
    pub fn new() -> Self {
        Self { title: SharedString::default() }
    }
}
```

- `slots` 为字符串数组字面量
- 保留名 `"default"` 对应模板内无 `name` 属性的 `<slot />`
- 不写 `slots` 参数 → 组件不接受任何插槽（父视图传 `<template slot>` 会被 validator 报 error）
- 宏自动为每个 slot 注入私有字段 `__rml_slot_<name>: Option<rml_core::slot::SlotRenderer>` 与 setter `__rml_set_slot_<name>`
  - `SlotRenderer` = `Box<dyn Fn(&mut Window, &mut App) -> AnyElement + Send + Sync + 'static>`
  - 用闭包而非直接存 `AnyElement`，因为 `IModel: Send + Sync` 要求组件线程安全，而 `AnyElement` 含 `Rc` 不满足 `Send`

**RML 模板侧**用 `<slot>` 占位符声明 slot 内容的渲染位置：

```html
<!-- components/card.rml -->
<component>
    <div class="card">
        <div class="card-header">
            <slot name="header" />
        </div>
        <div class="card-body">
            <slot />
        </div>
        <div class="card-footer">
            <slot name="footer" />
        </div>
    </div>
</component>
```

- `<slot name="header" />` 声明具名插槽位置
- `<slot />`（无 `name`）声明默认插槽位置（对应 `"default"`）
- codegen 将 `<slot>` 替换为 `self.__rml_slot_<name>.as_ref().map_or(gpui::Empty.into_any_element(), |f| f(_window, cx))`，调用闭包即时生成 element
- **`<slot>` 不支持默认内容**：`<slot>默认文本</slot>` 中的子节点会被忽略，未填充的插槽渲染为空

### ② 使用方：填充插槽

```html
<!-- 父视图 .rml -->
<Card title="My Card">
    <template slot="header">
        <h2>Card Title</h2>
        <Button label="Close" ghost="" />
    </template>

    <template slot="footer">
        <Button label="OK" primary="" />
    </template>

    <!-- 默认插槽：无 slot 属性的裸子节点（仅当 Card 声明了 "default"） -->
    <p>This is the card body content.</p>
</Card>
```

### 标准语法规则

| 场景 | 语法 | 说明 |
|------|------|------|
| 填充具名插槽 | `<template slot="name">...</template>` | `name` 必须在组件 `slots` 声明中，否则编译期 error |
| 填充默认插槽 | 裸子节点（无 `slot` 属性） | 仅当组件声明了 `"default"`；否则被忽略 + validator error |
| 自闭合空填充 | `<template slot="header"></template>` | 等价于不填充，渲染为空 |
| 未填充的插槽 | — | 渲染为空（`Option::None`） |

### 层次分明、简洁清晰的原则

- 具名插槽内容一律用 `<template slot="...">` 包裹，与 default 内容视觉分离
- 一个 `<template slot>` 内可放多节点（codegen 自动包裹 `div` 容器）
- default 内容放在所有 `<template slot>` 之后，作为"主内容"
- 不要在 `<slot>` 占位符内放内容（不支持默认内容）

## 6.3.4 属性齐全性保障

RML 通过**单一信源 + 双层校验**确保 codegen 属性映射齐全：

### 单一信源：`props_registry.rs`

`crates/engine/src/compiler/props_registry.rs` 是框架 codegen 翻译的唯一信源：

- `COMPONENT_PROPS`：扩展组件专用属性（Button / Input / Tree / MenuBar / status_bar ...）
- `SHELL_PROPS`：shell 根标签属性（window / tab_window / modern_window / component）
- `COMMON_STATIC_PROPS` / `COMMON_BIND_PROPS` / `COMMON_EVENT_PROPS`：通用属性
- 查询函数 `is_prop_registered(tag, attr)` / `is_shell_prop_registered(tag, attr)` 自动 kebab-case → PascalCase 规范化

### 双层校验

1. **编译期 error（用户拼写错误）** —— `validator.rs`：
   - shell 根标签的 bind/event 属性若不在 `SHELL_PROPS` → `ValidationError`
   - 扩展组件的 bind/event 属性若不在 `COMPONENT_PROPS` + 通用 → `ValidationError`
   - 用户组件的 `<template slot="x">` 中 `x` 若不在 `slots` 声明 → `ValidationError`

2. **codegen warning（框架开发者映射缺失）**：
   - `component_static_setter` / `component_bind_setter` 未命中分支：若 `is_prop_registered` 为 true → `eprintln!("[rml warning] ...")`
   - `gen_tab_window_wrapper` / `gen_modern_window_wrapper` 未命中分支：若 `is_shell_prop_registered` 为 true → warning
   - 提示开发者在对应 match 添加分支

### 维护规则

添加新组件或新属性时，**必须三同步**：

1. 在 `props_registry.rs` 的 `COMPONENT_PROPS` / `SHELL_PROPS` 中登记
2. 在 `component_bind_setter` / `component_static_setter` / `component_event_setter` 或 `shell.rs` 中添加对应 match 分支
3. 运行 `cargo test -p rust-rml-engine` 验证 `props_registry` 测试通过

详见 [属性映射参考](./reference/props-mapping.md)。

## 6.3.5 已知限制

- **slot 内容不应引用父视图 `self` 字段**：slot 内容被包装为 `SlotRenderer` 闭包（`move`），闭包不能捕获父视图 `self` 的引用（render 结束后 `&self` 失效）。需要向 slot 传递父视图数据时，应通过子组件自身的 props（pub 字段 + 绑定）传递，而非在 slot 内容中直接引用 `self.xxx`。`cx.t(...)`、`cx.current_theme()` 等不引用 `self` 的表达式可正常使用。
- **`<slot>` 不支持默认内容**：未填充的插槽渲染为空，无法在模板内指定 fallback 内容。

## 6.3.6 规划中特性

### 作用域插槽

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

## 6.3.7 小结

RML 插槽当前支持：

- **Shell 窗口插槽**：`<template slot="name">` 填充 tab_window / modern_window 的具名位置
- **自定义组件插槽**：`#[component(slots=[...])]` 契约 + `<slot>` 占位符 + `<template slot>` 填充
- **属性齐全性**：`props_registry` 单一信源 + validator 编译期 error + codegen warning 双层保障

规划中：

- 作用域插槽 `<slot let-item={item}>`

掌握插槽，你就能构建高复用、可配置的组件与窗口布局。

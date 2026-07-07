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
- 宏自动为每个 slot 在 `__rml_state.slots: HashMap<&'static str, SlotRenderer>` 中预留存储（通过 `__rml_state.slot(<name>)` 读取 `Option<&SlotRenderer>`），并生成 setter `__rml_set_slot_<name>`（内部调用 `self.__rml_state.set_slot("<name>", renderer)`）
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
- codegen 将 `<slot>` 替换为 `self.__rml_state.slot(<name>).map_or(gpui::Empty.into_any_element(), |f| f(_window, cx))`，调用闭包即时生成 element
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
- **作用域插槽的延迟调用受限**：`scope={panel}` 接收的 `panel: &dyn ISlotScope` 是渲染期引用，无法被 `'static` 闭包（如 `on-click`）捕获。`panel.maximize/restore/close` 等操作方法需在渲染期命令式调用（如 `render_bottom_panel` 内）。后续将通过 `to_op_handle()` API 扩展支持延迟调用。

## 6.3.6 作用域插槽（Scoped Slots）

### 概念

作用域插槽让 `<template slot="...">` 内容能够接收来自插槽宿主（slot host）的上下文参数，用于操控父容器（如 resizable）行为。普通插槽仅能渲染内容，作用域插槽还能"反向操控"宿主。

### 语法

`<template slot="bottom" scope={panel}>...</template>`

- `scope={name}` 中 `name` 为接收 `&dyn ISlotScope` 的变量名
- `name` 必须为简单标识符（不能是 `foo.bar` / `foo(1)`）
- 不写 `scope={...}` 时，插槽首参以 `_scope` 忽略，向后兼容
- `scope` 仅可在 `<template slot="...">` 上使用，普通元素无效

### ISlotScope API

作用域变量类型为 `&dyn rml_core::slot::ISlotScope`，提供以下方法：

| 方法 | 返回 | 说明 |
|------|------|------|
| `slot_name()` | `&str` | 插槽名（"left"/"right"/"bottom"/...） |
| `current_size()` | `Option<Pixels>` | 当前尺寸（left/right 为宽度，bottom 为高度） |
| `container_size()` | `Option<Pixels>` | 容器总尺寸（用于 maximize 计算） |
| `has_resizable()` | `bool` | 是否支持 resizable 操控 |
| `maximize(window, cx)` | `()` | 最大化此面板（记录原尺寸供 restore 还原） |
| `restore(window, cx)` | `()` | 还原到 maximize 之前的尺寸 |
| `close(window, cx)` | `()` | 关闭/折叠此面板（尺寸调为 0 或最小阈值） |

### 实现方

- `NullSlotScope`：默认空作用域，所有方法返回 `None` / no-op
  - 自定义组件的 `<slot>` 占位符默认传此类型
  - `menu`/`title`/`footer`/`tabs` 等 shell slot 也使用此类型（无 resizable 操控）
- `TabWindowSlotScope`：TabWindow 的 left/right/bottom 插槽，暴露 resizable 操控权
  - 在 `TabWindowShell::render` 中通过 `use_keyed_state` 持久化 `prev_size`，跨渲染保存 maximize 前的尺寸

### 使用示例

```html
<tab-window title="..." left-size={left_size}>
    <template slot="bottom" scope={panel}>
        <component content={self.render_bottom_panel(panel, _window, cx)} />
    </template>
</tab-window>
```

```rust
impl MainWindow {
    pub fn render_bottom_panel(
        &self,
        panel: &dyn rml_core::slot::ISlotScope,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> gpui::AnyElement {
        let slot_name = panel.slot_name();
        let current_size = panel.current_size();
        let has_resizable = panel.has_resizable();
        // 在此可调用 panel.maximize(_window, _cx) / restore / close
        // ...
    }
}
```

### Codegen 路径

1. **Parser**：`scope={name}` 走默认分支，解析为 `Attribute::Bind { name: "scope", expr: "name" }`
2. **Validator**：校验 `scope` 仅在 `<template slot="...">` 上使用，且必须为简单标识符；在无 resizable 的 slot 上写 scope 仅警告
3. **Codegen (shell.rs)**：`extract_scope` 提取 scope 变量名；`wrap_shell_slot(slot_code, scope_var)` 生成闭包，闭包内 `let {name}: &dyn ISlotScope = scope;`
4. **Codegen (render.rs)**：`gen_slot_code!` 宏将 scope_var 作为 `loop_vars` 传入 `gen_node`，使 slot 内容表达式可解析 `panel` 标识符
5. **Runtime (tab_window.rs)**：`TabWindowShell::render` 构造 `TabWindowSlotScope`，通过 `use_keyed_state` 持久化 `prev_size`

### 限制

- `menu`/`title`/`footer`/`tabs` 等 shell slot 不支持 resizable 操控（`has_resizable()` 返回 false）
- 自定义组件的 `<slot>` 默认传 `NullSlotScope`，不暴露父容器操控权
- `panel.maximize/restore/close` 为渲染期方法，需在 `render_*` 等命令式方法内调用。在 `on-click` 等 `'static` 闭包中延迟调用需要 `to_op_handle()` API（规划中）

## 6.3.7 小结

RML 插槽当前支持：

- **Shell 窗口插槽**：`<template slot="name">` 填充 tab_window / modern_window 的具名位置
- **自定义组件插槽**：`#[component(slots=[...])]` 契约 + `<slot>` 占位符 + `<template slot>` 填充
- **属性齐全性**：`props_registry` 单一信源 + validator 编译期 error + codegen warning 双层保障
- **作用域插槽**：`<template slot="..." scope={name}>` 接收 `&dyn ISlotScope`，支持 resizable 操控（TabWindow 的 left/right/bottom）

规划中：

- `to_op_handle()` API：让 `on-click` 等 `'static` 闭包也能延迟调用 `maximize/restore/close`

掌握插槽，你就能构建高复用、可配置的组件与窗口布局。

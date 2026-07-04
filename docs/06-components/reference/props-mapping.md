# 组件属性映射参考

> RML 属性 ↔ gpui-component builder 方法对照表。本表由 `crates/engine/src/compiler/props_registry.rs` 维护，是框架 codegen 翻译的单一信源。

## 维护规则

添加新组件或新属性时，**必须三同步**：

1. 在 `crates/engine/src/compiler/props_registry.rs` 的 `COMPONENT_PROPS` / `SHELL_PROPS` 中登记
2. 在 `component_bind_setter` / `component_static_setter` / `component_event_setter` 或 `shell.rs` 中添加对应 match 分支
3. 运行 `cargo test -p rust-rml-engine` 验证 `props_registry` 测试通过

### Tag 规范化

`is_prop_registered(tag, attr)` / `is_shell_prop_registered(tag, attr)` 查询时通过 `canonical_tag()` 规范化标签：
- kebab-case → PascalCase（如 `menu-bar` → `MenuBar`、`status_bar` → `StatusBar`）
- 小写别名 → PascalCase（如 `accordion` → `Accordion`、`item` → `AccordionItem`）

因此在 `COMPONENT_PROPS` 中登记的 tag 用 PascalCase 即可，`<accordion>` / `<item>` / `<accordion-item>` / `<Accordion>` / `<AccordionItem>` 五种写法都能命中同一注册条目，无需重复登记。

## 属性齐全性双层保障

RML 通过两层机制确保 codegen 属性映射齐全：

1. **编译期 error（用户拼写错误）** —— `crates/engine/src/compiler/validator.rs`：
   - shell 根标签的 bind/event 属性若不在 `SHELL_PROPS` → `ValidationError`
   - 扩展组件的 bind/event 属性若不在 `COMPONENT_PROPS` + 通用属性 → `ValidationError`
   - 用户组件的 `<template slot="x">` 中 `x` 若不在组件 `slots` 声明 → `ValidationError`

2. **codegen warning（框架开发者映射缺失）**：
   - `component_static_setter` / `component_bind_setter` 未命中分支：若 `is_prop_registered` 为 true → `eprintln!("[rml warning] ...")`
   - `gen_tab_window_wrapper` / `gen_modern_window_wrapper` 未命中分支：若 `is_shell_prop_registered` 为 true → warning
   - 提示开发者在对应 match 添加分支

未在注册表登记的属性会在 validator 阶段报 error（bind/event）；已登记但无 match 分支的属性会输出 `[rml warning]` 提示。

## 通用属性（所有 Stateless / Stateful 组件共享）

### 静态属性 `attr="value"`

| RML 属性 | 生成的 builder 方法 | 说明 |
|----------|---------------------|------|
| `label="..."` | `.label("...")` | 文本标签 |
| `placeholder="..."` | `.placeholder("...")` | 输入占位符 |
| `tooltip="..."` | `.tooltip("...")` | 工具提示 |
| `primary` / `secondary` / `danger` / `success` / `warning` / `info` / `ghost` / `link` / `text` | `.primary()` 等 | Button variant |
| `size="xsmall"` / `size="small"` / `size="medium"` / `size="large"` | `.with_size(Size::*)` | Sizable 尺寸 |
| `compact` / `loading` | `.compact()` 等 | 状态 |
| `disabled="true"` | `.disabled(true)` | 禁用 |
| `selected="true"` | `.selected(true)` | 选中 |
| `font_thin` ... `font_black` | `.font_bold()` 等 | StyledExt 字体权重 |
| `h_flex` / `v_flex` | `.h_flex()` 等 | StyledExt 布局 |

### 绑定属性 `attr={expr}`

| RML 属性 | 生成的 builder 方法 | 说明 |
|----------|---------------------|------|
| `content={expr}` | `.child(expr)` | 直接嵌入 AnyElement |
| `value={field}` | `.value(self.field.clone())` | 值绑定 |
| `disabled={cond}` | `.disabled(cond)` | 禁用条件 |
| `selected={cond}` | `.selected(cond)` | 选中条件 |
| `checked={cond}` | `.selected(cond)` | 复选状态（映射到 selected） |
| `label={expr}` | `.label(self.expr.clone())` | 标签绑定 |

### 事件属性 `on*={fn}`

| RML 属性 | 生成的 builder 方法 | 适用组件 |
|----------|---------------------|----------|
| `on-click={fn}` | `.on_click(cx.listener(...))` | 所有组件 |
| `onchange={fn}` | `.on_change(cx.listener(...))` | Input / TextInput |
| `on_activate={fn}` | `.on_activate_rc(Rc::new(...))` | Tree |

## 组件专用属性

| 组件 | 专用属性 | 说明 |
|------|----------|------|
| `Input` / `TextInput` | `onchange` | 输入变化事件 |
| `Tree` | `items`, `on_activate`, `on_select` | 树数据与事件 |
| `MenuBar` / `menu` | `items` | 菜单项数据绑定 |
| `status_bar` | `items` | 状态栏项数据绑定 |
| `Accordion` | `multiple`, `bordered`, `on_toggle_click` | 多选/边框/切换事件 |
| `AccordionItem` | `title`, `open`, `icon` | 子项标题/初始展开/图标 |
| `DescriptionList` | `vertical`, `bordered`, `columns`, `label-width`, `size`, `items` | 布局方向/边框/列数/标签列宽/尺寸/批量数据绑定 |
| `DescriptionItem` | `label`, `value`, `span`, `size` | 子项标签（构造器参数）/值/跨列/尺寸 |

## Shell 窗口属性

> `.rml` 中使用 kebab-case（如 `selected-tab`、`show-chrome`），parser 的 `normalize_attr_name()` 自动转换为 snake_case 供内部查找。

### `<tab_window>`

| 属性 | 类型 | 说明 |
|------|------|------|
| `title` | static | 窗口标题 |
| `width` / `height` | static | 窗口尺寸 |
| `startup` | static | 启动位置（如 `CenterScreen`） |
| `icon` | bind | 图标（`IconName::Frame`） |
| `tabs` | bind | TabBar 项列表 |
| `selected-tab` | bind | 当前选中 tab 索引 |
| `show-chrome` | bind | 是否显示窗口 chrome |
| `left-size` / `right-size` / `bottom-size` | bind | 面板尺寸 |
| `on-tab-click` | event | Tab 点击事件 |
| `on-chrome-toggle` | event | Chrome 切换事件 |

### `<modern_window>`

| 属性 | 类型 | 说明 |
|------|------|------|
| `title` | static | 窗口标题 |
| `width` / `height` | static | 窗口尺寸 |
| `startup` | static | 启动位置 |
| `icon` | bind | 图标 |
| `menu` | bind | 菜单数据 |
| `footer` | bind | 状态栏数据 |

### `<window>`

| 属性 | 类型 | 说明 |
|------|------|------|
| `title` | static | 窗口标题 |
| `width` / `height` | static | 窗口尺寸 |
| `startup` | static | 启动位置 |
| `icon` | bind | 图标 |

## Shell 插槽（`<template slot="name">`）

| Shell | 插槽名 | builder 方法 | 用途 |
|-------|--------|--------------|------|
| tab_window / modern_window | `menu` | `.menu_slot(...)` | 菜单栏 |
| tab_window / modern_window | `title` | `.title_ext_slot(...)` | 标题扩展区 |
| tab_window / modern_window | `footer` | `.status_slot(...)` | 状态栏 |
| tab_window | `left` | `.slot_left(...)` | 左侧面板 |
| tab_window | `right` | `.slot_right(...)` | 右侧面板 |
| tab_window | `bottom` | `.slot_bottom(...)` | 底部面板 |

## 自定义组件插槽

自定义组件通过 `#[component(slots = [...])]` 宏参数声明插槽契约，模板内用 `<slot>` 占位符，父视图用 `<template slot="...">` 填充：

```rust
#[component(slots = ["header", "default", "footer"])]
pub struct Card { ... }
```

| 角色 | 语法 | 说明 |
|------|------|------|
| 契约声明 | `#[component(slots = ["header", "default", "footer"])]` | Rust 侧声明组件接受的插槽列表 |
| 占位符 | `<slot name="header" />` / `<slot />` | 模板内声明渲染位置；无 `name` 对应 `"default"` |
| 填充具名 | `<template slot="header">...</template>` | `name` 必须在 `slots` 声明中，否则 validator error |
| 填充默认 | 裸子节点（无 `slot` 属性） | 仅当声明了 `"default"`；否则 validator error |

- `<slot>` 不支持默认内容，未填充的插槽渲染为空
- codegen 将 `<slot>` 替换为 `self.__rml_state.slot(<name>).map_or(gpui::Empty.into_any_element(), |f| f(_window, cx))`
- slot 渲染闭包存储于 `__rml_state.slots: HashMap<&'static str, SlotRenderer>`，每次 render 通过 `__rml_state.slot(<name>)` 查询调用（不消费，可重复渲染）

详见 [6.3 插槽与内容分发](../slots.md)。

## 相关文档

- [插槽与内容分发](../slots.md)
- [窗口根元素](./window-roots.md)
- [代码生成](../../10-advanced/code-generation.md)

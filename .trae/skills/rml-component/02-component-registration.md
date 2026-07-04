# 02 组件注册

## 三处同步协议

新增组件时，**必须同步**以下三处：

### 1. `crates/engine/src/tags.rs::component_lookup`

注册组件的构造路径和 ComponentKind：

```rust
"TabBar" | "tab-bar" => Some(ComponentTag {
    ctor_path: "rml_ui::TabBar",
    kind: ComponentKind::StatelessWithItems,
}),
```

### 2. `crates/engine/src/compiler/props_registry.rs`

注册组件的所有属性（专用属性，不含通用属性）：

```rust
("TabBar", &[
    "selected_index", "on_click", "prefix", "suffix", "last_empty_space",
    "menu", "track_scroll",
    "underline", "pill", "flat", "outline", "segmented",
]),
```

### 3. 各组件 `setters.rs`

实现属性到 builder 方法的映射：
- `static_setter(name, value, tag)` → 静态属性
- `bind_setter(name, expr, loop_vars, computed, tag)` → 绑定属性
- `event_setter(name, handler, tag)` → 事件属性

## ComponentKind 枚举

| Kind | 构造形式 | 代表组件 |
|------|----------|----------|
| `Stateless` | `Button::new(id)` | Button, Card, MenuBar |
| `StatelessNoId` | `TitleBar::new()` | TitleBar, StatusBar, Avatar |
| `StatelessWithItems` | `Accordion::new(id)` + 闭包/直接 .child() | Accordion, TabBar, Table, DescriptionList |
| `Stateful { state_field }` | `Input::new(&self.input_state)` | Input, TextInput, CodeEditor, Tree |
| `EntityRef` | `self.field.as_ref().expect(...).clone()` | ActivityBar |

### StatelessWithItems 子类型差异

| 组件 | 子节点注入方式 | 子标签 |
|------|----------------|--------|
| Accordion | `.item(\|item\| item...)` 闭包 | AccordionItem / item |
| TabBar | `.child(Tab::new()...)` 直接 | Tab / tab, TabItem / tab-item |
| Table | `.columns(...)` + `.header_template(...)` | Column / column, `<template slot="...">` |
| DescriptionList | `.children(...)` 批量 | DescriptionItem / description |

## 子标签识别

`is_item_builder_tag(tag)` 识别 item builder 子标签（不在 `component_lookup` 中注册）：

| PascalCase | 小写别名 | kebab-case | 父组件 |
|------------|----------|------------|--------|
| AccordionItem | item | accordion-item | Accordion |
| Tab | tab | — | TabBar |
| TabItem | — | tab-item | TabBar |
| Column | column | — | Table |
| DescriptionItem | description | — | DescriptionList |
| DescriptionSeparator | separator | — | DescriptionList |

**设计理由**：item builder 仅在父组件内合法，不注册到 `component_lookup` 避免被误用为顶层组件。

## 窗口外壳注册

根标签在 `is_root_tag` / `root_tag_lookup` 中注册：

| tag | RootTag | codegen 路径 |
|-----|---------|--------------|
| `<window>` | Window | 基础窗口 |
| `<modern-window>` | ModernWindow | `gen_modern_window_wrapper` |
| `<tab-window>` | TabWindow | `gen_tab_window_wrapper` |
| `<dialog>` | DialogWindow | 对话框方法生成 |
| `<component>` | Component | 可复用组件 |

**注意**：`ModernWindowShell` / `TabWindowShell` 不在 `component_lookup` 中注册，它们由 codegen 根元素处理路径直接生成包裹代码。

## 窗口外壳属性注册

`SHELL_PROPS` 注册窗口外壳的 bind 属性，供 `is_shell_prop_registered` 校验：

```rust
("modern-window", &["menu", "footer", "icon"]),
("tab-window", &["menu", "footer", "icon", "tabs", "selected_index", ...]),
```

**key 规范**：使用 kebab-case tag 名作为 key（`"modern-window"` / `"tab-window"`），不用 snake_case。

## StatusBar 命名冲突

`StatusBar` 在 `component_lookup` 中有两个独立条目：

```rust
// RML MVVM 状态栏（支持 items={...} 绑定）
"StatusBar" | "status-bar" => Some(ComponentTag {
    ctor_path: "rml_ui::StatusBar",
    kind: ComponentKind::StatelessNoId,
}),

// gpui-component 原生状态栏（手动 .left() / .right() 组装）
"NativeStatusBar" | "native-status-bar" => Some(ComponentTag {
    ctor_path: "rml_ui::NativeStatusBar",
    kind: ComponentKind::StatelessNoId,
}),
```

`props_registry.rs` 中 `StatusBar` 注册 `items` 属性，`NativeStatusBar` 无 `items`（手动组装）。

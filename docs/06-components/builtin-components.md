# 6.1 内置组件

> **本节目标**：准确了解 RML 当前支持的标签体系，并跳转到逐组件参考文档。

## 6.1.1 双轨标签策略

RML 组件来自两个互不混淆的层次：

**扩展组件标签规范**：RML 中推荐使用 **kebab-case**（如 `<context-menu>`、`<menu-item>`），`crates/engine/src/tags.rs` 的 `normalize_component_tag()` 在 codegen 时映射为 PascalCase（`ContextMenu`、`MenuItem`）。PascalCase 写法仍兼容。特殊 snake_case 标签（`menu`、`status_bar`）保持原样。

```
┌─────────────────────────────────────────────────────────┐
│                    RML 可用标签                           │
│                                                         │
│  ┌─────────────────────┐   ┌─────────────────────────┐ │
│  │  基础轨（小写 HTML）  │   │  扩展轨（路由表注册）     │ │
│  │  BuiltinTag         │   │  component_lookup()     │ │
│  │                     │   │                         │ │
│  │  div, span, input   │   │  Button, Tree, menu     │ │
│  │  h1~h6, ul, li …    │   │  ActivityBar, status_bar│ │
│  └─────────────────────┘   └─────────────────────────┘ │
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │  根节点：window / modern_window / tab_window /      ││
│  │          dialog / component                         ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

**权威来源**：`crates/engine/src/tags.rs` 中的 `lookup()`、`component_lookup()`、`root_tag_lookup()`。

未在路由表注册的 gpui-component 类型（如 Modal、Navbar、Table、Tabs 等）**不能**在 `.rml` 中作为标签使用，即使 gpui-component 库本身提供这些类型。

## 6.1.2 扩展轨组件一览

完整属性与示例见 **[组件参考目录](./reference/INDEX.md)**。

### 表单

| RML 标签 | 文档 |
|----------|------|
| `Button` | [button.md](./reference/button.md) |
| `ButtonGroup` | [button-group.md](./reference/button-group.md) |
| `Badge` | [badge.md](./reference/badge.md) |
| `Checkbox` | [checkbox.md](./reference/checkbox.md) |
| `Label` | [label.md](./reference/label.md) |
| `Input` / `TextInput` | [input.md](./reference/input.md) / [text-input.md](./reference/text-input.md) |
| `Slider` | [slider.md](./reference/slider.md) |
| `Switch` | [switch.md](./reference/switch.md) |
| `Tag` | [tag.md](./reference/tag.md) |
| `Progress` / `ProgressCircle` | [progress.md](./reference/progress.md) / [progress-circle.md](./reference/progress-circle.md) |
| `Separator` | [separator.md](./reference/separator.md) |

### 布局 / Shell

| RML 标签 | 文档 |
|----------|------|
| `TitleBar` | [title-bar.md](./reference/title-bar.md) |
| `StatusBar` | [gpui-status-bar.md](./reference/gpui-status-bar.md) |
| `status_bar` | [status-bar.md](./reference/status-bar.md) |
| `ActivityBar` | [activity-bar.md](./reference/activity-bar.md) |

### 数据 / 导航

| RML 标签 | 文档 |
|----------|------|
| `Tree` | [tree.md](./reference/tree.md) |
| `menu` / `MenuBar` | [menu.md](./reference/menu.md) |
| `ContextMenu` | [context-menu.md](./reference/context-menu.md) |
| `DropdownMenu` | [dropdown-menu.md](./reference/dropdown-menu.md) |
| `MenuItem` / `MenuSeparator` | [menu-items.md](./reference/menu-items.md) |
| `AppMenuBar` | [app-menu-bar.md](./reference/app-menu-bar.md) |
| `accordion` / `Accordion` | [accordion.md](./reference/accordion.md) |

## 6.1.3 基础轨 HTML 标签

| 标签 | 说明 |
|------|------|
| `div` / `span` / `p` | 容器与文本 |
| `h1`–`h6` | 标题（内置字号） |
| `button` | 基础 `div()` 占位，**无** Button 组件样式 |
| `input` / `textarea` | 支持 `model={field}` 双向绑定 → `rml_ui::Input` |
| `ul` / `ol` / `li` | 列表布局 |
| `img` / `a` / `label` / `br` | 基础占位 |

详见 [builtin-html.md](./reference/builtin-html.md)。

## 6.1.4 根节点

| 根标签 | 用途 |
|--------|------|
| `window` | 基础窗口 |
| `modern_window` | 现代窗口外壳 |
| `tab_window` | TabBar + 多插槽（Demo 主窗口） |
| `dialog` | 模态对话框 |
| `component` | 可复用 `#[component]` 片段 |

详见 [window-roots.md](./reference/window-roots.md)。

## 6.1.5 快速对照：该用哪个标签？

| 需求 | 正确写法 | 错误写法 |
|------|----------|----------|
| 带样式的按钮 | `<Button primary="" on-click={...}>` | `<button variant="primary">` |
| 双向文本输入 | `<input model={name}>` | `<Input model={name}>` |
| 状态栏 MVVM | `<status_bar items={status_items}>` | `<StatusBar items={...}>` |
| 案例树 | `<Tree on_activate={...}>` + Rust 初始化 `case_tree_state` | `<Tree items={...}>` |
| 模态框 | `<dialog>` + `open(window, cx)` | `<Modal>`（未注册） |

## 6.1.6 codegen 属性支持范围

扩展组件的属性由 `crates/engine/src/compiler/component.rs` 中的三个映射函数决定：

- **静态**：`label`、`placeholder`、`primary`/`ghost`/…、`disabled`、`small` 等
- **绑定**：`value`、`disabled`、`selected`、`label`；`ActivityBar` 的 `panels`/`actions`；`menu`/`MenuBar`/`status_bar` 的 `items`
- **事件**：`on-click`（通用）、`onchange`（Input/TextInput）、`on_panel_change`（ActivityBar）、`on_activate`（Tree）
- **菜单**：`ContextMenu` / `DropdownMenu` / `MenuBar` 由 `compiler/menu/` codegen；子项仅 `MenuItem` + `MenuSeparator`（见 [menu-items.md](./reference/menu-items.md)）

## 6.1.7 小结

- 内置组件 ≠ gpui-component 全量导出；以路由表为准。
- 逐组件 API 请查阅 [reference/INDEX.md](./reference/INDEX.md)。
- Shell 类控件配合 [贡献点架构](../09-architecture/contribution-system.md) 与 demo 的 `map_shell_chrome` 使用。

下一节 → [6.2 自定义组件](./custom-components.md)

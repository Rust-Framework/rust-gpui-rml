# 组件参考目录

> 本目录是 RML **已注册组件**的权威参考。仅收录 `crates/engine/src/tags.rs` 中 `component_lookup()` 与根节点路由表实际支持的标签；未在路由表中的 gpui-component 类型**不能**在 `.rml` 中直接使用。

## 根节点

| 文档 | RML 标签 | 说明 |
|------|----------|------|
| [window-roots.md](./window-roots.md) | `window` / `modern_window` / `tab_window` / `dialog` / `component` | 窗口外壳与可复用组件根 |

## 表单（Form）

| 文档 | RML 标签 | 构造类型 |
|------|----------|----------|
| [button.md](./button.md) | `Button` | Stateless |
| [button-group.md](./button-group.md) | `ButtonGroup` | Stateless |
| [avatar.md](./avatar.md) | `Avatar` / `AvatarGroup` | StatelessNoId |
| [badge.md](./badge.md) | `Badge` | Stateless |
| [checkbox.md](./checkbox.md) | `Checkbox` | Stateless |
| [label.md](./label.md) | `Label` | Stateless |
| [input.md](./input.md) | `Input` | Stateful（`input_state`） |
| [text-input.md](./text-input.md) | `TextInput` | Stateful（同 `Input`） |
| [code-editor.md](./code-editor.md) | `CodeEditor` | Stateful（`editor_state`，基于 Input 多行代码编辑器） |
| [slider.md](./slider.md) | `Slider` | Stateless |
| [switch.md](./switch.md) | `Switch` | Stateless |
| [tag.md](./tag.md) | `Tag` | Stateless |
| [progress.md](./progress.md) | `Progress` | Stateless |
| [progress-circle.md](./progress-circle.md) | `ProgressCircle` | Stateless |
| [separator.md](./separator.md) | `Separator` | Stateless |

## 布局 / Shell（Layout / Shell）

| 文档 | RML 标签 | 构造类型 |
|------|----------|----------|
| [title-bar.md](./title-bar.md) | `TitleBar` | StatelessNoId（容器） |
| [gpui-status-bar.md](./gpui-status-bar.md) | `StatusBar` | StatelessNoId（容器，gpui-component 原生） |
| [status-bar.md](./status-bar.md) | `status_bar` | StatelessNoId（MVVM 绑定包装） |
| [activity-bar.md](./activity-bar.md) | `ActivityBar` | Stateless（容器 + 数据绑定） |
| [tabs.md](./tabs.md) | `Tabs` / `Tab` | Stateless（WPF TabControl：header + body） |
| [tab-bar.md](./tab-bar.md) | `TabBar` / `Tab` | Stateless（原生 header-only 标签栏） |
| [icon.md](./icon.md) | —— | `IconSpec` 贡献点图标规格（跨组件基础设施） |

## 数据 / 导航（Data / Navigation）

| 文档 | RML 标签 | 构造类型 |
|------|----------|----------|
| [tree.md](./tree.md) | `Tree` | Stateful（`case_tree_state`） |
| [menu.md](./menu.md) | 菜单索引 | codegen `compiler/menu/` |
| [menu-bar.md](./menu-bar.md) | `MenuBar` / `menu` | codegen + `items` 绑定 |
| [context-menu.md](./context-menu.md) | `ContextMenu` | codegen |
| [dropdown-menu.md](./dropdown-menu.md) | `DropdownMenu` | codegen |
| [menu-items.md](./menu-items.md) | `MenuItem` / `MenuSeparator` | 菜单子项 |
| [app-menu-bar.md](./app-menu-bar.md) | `AppMenuBar` | codegen |
| [accordion.md](./accordion.md) | `accordion` / `Accordion` | StatelessWithItems（闭包 builder） |
| [description-list.md](./description-list.md) | `descriptions` / `DescriptionList` | StatelessWithItems（直接 `.child()` 注入） |

## 内置 HTML 标签

| 文档 | 标签范围 | 说明 |
|------|----------|------|
| [builtin-html.md](./builtin-html.md) | `div` / `span` / `p` / `h1`–`h6` / `button` / `input` / `textarea` / `ul` / `ol` / `li` / `img` / `a` / `label` / `br` | 基础轨；`input`/`textarea` 支持 `model` 双向绑定 |

## 阅读顺序建议

1. 先读 [window-roots.md](./window-roots.md) 了解根节点与插槽分区。
2. 表单类从 [button.md](./button.md) 与 [input.md](./input.md) 入手。
3. Shell 应用读 [activity-bar.md](./activity-bar.md)、[menu.md](./menu.md)、[status-bar.md](./status-bar.md)、[tree.md](./tree.md)，配合 [贡献点架构](../../09-architecture/contribution-system.md)。

## codegen 与 gpui-component 的差距

扩展组件属性由 `component.rs` 映射；**菜单**由 `compiler/menu/` 直译 gpui-component `PopupMenu` API（`ContextMenu`、`DropdownMenu`、`MenuBar`、`MenuItem`）。未覆盖的能力可在 Rust code-behind 手写。

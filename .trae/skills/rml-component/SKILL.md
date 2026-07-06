---
name: rml-component
description: RML 声明式 UI 框架组件支持规范。统一组件命名、注册、属性分类、数据绑定、插槽、CSS 定制、尺寸布局的开发范式，确保新组件开发遵循一致的架构约定。
when_to_apply: 在 RML 框架内新增组件、修改组件属性、调整数据绑定、扩展插槽、修改 CSS 选择器、或审查组件范式一致性时使用。
---

# RML 组件支持规范

本 Skill 是 RML 框架组件开发的权威规范，覆盖 8 个维度：
1. 命名规范（声明式 kebab-case / 内部 snake_case 双层模型）
2. 组件注册（三处同步协议）
3. 属性分类（static / bind / event 三类，组件专用 / 通用 / 警告丢弃 三级）
4. 数据绑定（children / items={expr} / {each} / model={field} 四种形式）
5. 插槽模板（基础插槽 / Table 专用模板 / Scoped slot）
6. CSS 定制（选择器父链匹配 / 主题变量）
7. 尺寸布局（size=medium / vertical=true / variant 快捷方法）
8. 图标处理（IconSpec：Named / Path / Url 三 variant + 嵌入资源集成）

## 核心设计原则

- **声明式强制 kebab-case**：`.rml` 文件中所有 tag 名、属性名使用 kebab-case（`<tab-bar on-click={...} size="small">`）
- **内部 snake_case**：Rust 代码中 builder 方法、字段名使用 snake_case（`.on_click(...)` / `self.input_state`）
- **单一信源**：`props_registry.rs` 是组件属性的唯一信源，三处同步协议保证一致性
- **不保留兼容性设计**：框架全新开发，拒绝补丁式代码，无法容忍双形式并存
- **medium 不用 middle**：`size=medium` 表示中等大小，使用 `medium` 名称
- **vertical 不重复 horizontal**：`vertical=true` 表示纵向，默认横向，不提供 `horizontal` 属性
- **最佳实践优先**：站在设计者和架构师视角，遵循 Rust idiomatic 风格

## Quick Reference

| 维度 | 规范 | 参考文档 |
|------|------|----------|
| 命名 | 声明式 kebab-case，内部 snake_case | [01-naming-conventions.md](01-naming-conventions.md) |
| 注册 | tags.rs + props_registry.rs + setters.rs 三处同步 | [02-component-registration.md](02-component-registration.md) |
| 属性 | static/bind/event 三类，组件专用→通用→警告丢弃 | [03-property-classification.md](03-property-classification.md) |
| 绑定 | children / items={expr} / {each} / model={field} | [04-data-binding.md](04-data-binding.md) |
| 插槽 | `<template slot="name">`，Table 专用模板，scoped slot | [05-slot-template.md](05-slot-template.md) |
| CSS | Class/Id/Tag/Universal/Compound/Descendant/Child 选择器 | [06-css-customization.md](06-css-customization.md) |
| 尺寸 | size=xsmall\|small\|medium\|large，vertical=true | [07-size-layout-conventions.md](07-size-layout-conventions.md) |
| 图标 | `IContribution::icon() -> Option<IconSpec>`；Named/Path/Url；Path 经 CompositeAssets 透明支持嵌入资源 | [09-icon-handling.md](09-icon-handling.md) |

## 支持的组件清单

| ComponentKind | 组件 |
|---------------|------|
| Stateless | Button, ButtonGroup, Badge, Checkbox, Label, Separator, Tag, Progress, ProgressCircle, Slider, Switch, Card, MenuBar (menu-bar, menu) |
| StatelessNoId | TitleBar, NativeStatusBar (native-status-bar), StatusBar (status-bar), Avatar, AvatarGroup |
| StatelessWithItems | DescriptionList (descriptions), Accordion (accordion), TabBar (tab-bar), Table (table) |
| Stateful | Input, TextInput, CodeEditor, Tree |
| EntityRef | ActivityBar |

**根标签**：`<window>` / `<modern-window>` / `<tab-window>` / `<dialog>` / `<component>`

**Item builder 子标签**（不在 component_lookup 中注册）：
- AccordionItem (item, accordion-item)
- Tab (tab), TabItem (tab-item)
- Column (column)
- DescriptionItem (description), DescriptionSeparator (separator)

## 新组件开发检查清单

新增组件时，参考 [08-new-component-checklist.md](08-new-component-checklist.md) 的 12 项检查清单，确保范式一致性。

## 反模式速查

以下写法**已废弃/禁止**：
- `<tab_window>` / `<modern_window>` / `<status_bar>` / `<tab_bar>`（snake_case tag）
- `size=middle`（应为 `size=medium`）
- `horizontal={true}`（不提供 horizontal，默认横向）
- `<TabBar on_click={...}>`（应为 `on-click`）
- `onclick={...}`（应为 `on-click`）

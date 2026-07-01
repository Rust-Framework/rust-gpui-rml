# StatusBar

## 概述

PascalCase `<StatusBar>` 路由到 gpui-component `rml_ui::StatusBar`，**StatelessNoId** **容器**组件。子节点作为状态栏内容。

> Shell 场景的 MVVM 绑定应使用小写 [status_bar](./status-bar.md)（`RmlStatusBar` + `items={...}`）。

## 基本用法

```html
<StatusBar>
    <Label label="就绪" />
    <Label label={status_text} />
</StatusBar>
```

## 属性

codegen 无 StatusBar 专用属性；通用属性大多无意义。

## 事件

由子组件处理。

## 数据绑定

通过子节点实现动态内容，例如：

```html
<StatusBar>
    <Label label={left_status} />
    <Label label={right_status} />
</StatusBar>
```

## 子节点 / 插槽

**容器组件**：子节点传入状态栏区域。

## 完整示例

```html
<modern_window title="工具" width="800" height="600">
  <slot_footer>
    <StatusBar>
        <Label label={connection_status} />
    </StatusBar>
  </slot_footer>
</modern_window>
```

## 常见错误

1. **写 `items={status_items}`** — 应使用 `<status_bar items={...}>`。
2. **与 `RmlStatusBar` 混淆** — 本组件是 gpui-component 原生容器，无 MVVM items API。

## 相关组件

- [status-bar.md](./status-bar.md) — MVVM `status_bar` 标签
- [window-roots.md](./window-roots.md)

## RML 未覆盖的 API

`.left()` / `.right()` 链式布局需在 Rust 中调用 gpui-component builder。

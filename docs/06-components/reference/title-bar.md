# TitleBar

## 概述

`TitleBar` 路由到 gpui-component `rml_ui::TitleBar`，**StatelessNoId** 容器组件。用于手动组装窗口标题栏区域，子节点作为标题栏内容传入。

> 使用 `tab_window` / `modern_window` 根时，标题栏通常由 Shell 自动提供；`TitleBar` 用于自定义布局场景。

## 基本用法

```html
<TitleBar>
    <Label label="自定义标题区" />
</TitleBar>
```

## 属性

通用静态属性可能部分生效，但 TitleBar 主要通过子节点承载内容。codegen **无** TitleBar 专用属性映射。

## 事件

子组件各自处理事件；TitleBar 本身无专用 RML 事件。

## 数据绑定

通过子节点绑定实现，如嵌套 `<Label label={title} />`。

## 子节点 / 插槽

**容器组件**：所有子节点通过 `.child()` / `.children()` 传入 TitleBar。

## 完整示例

```html
<modern_window title="应用" width="900" height="600">
    <template slot="title">
        <TitleBar>
            <Label label={window_subtitle} font_semibold="" />
        </TitleBar>
    </template>
    <div class="main">...</div>
</modern_window>
```

## 常见错误

1. **与 `tab_window` 自带 TabBar 混淆** — Tab 标题由 `tabs={...}` 驱动，不是 `TitleBar`。
2. **期望拖动/关闭按钮** — 需 gpui-component TitleBar builder 或 Shell 提供。

## 相关组件

- [window-roots.md](./window-roots.md)
- [gpui-status-bar.md](./gpui-status-bar.md)

## RML 未覆盖的 API

gpui-component TitleBar 的窗口控制按钮、拖动区域等需 Rust 手写。

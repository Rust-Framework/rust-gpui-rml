# Separator

## 概述

`Separator` 路由到 `rml_ui::Separator`，**Stateless** 组件，用于水平或垂直分隔线。

## 基本用法

```html
<Separator />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 若 API 支持带文字分隔 |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| 其他通用属性 | — | — | `small`、`font_*` 等可能无视觉效果 |

## 事件

| 事件 | 说明 |
|------|------|
| `on-click` | 一般不用于分隔线 |

## 数据绑定

通常无需绑定。

## 子节点 / 插槽

不支持有意义的子节点。

## 完整示例

```html
<div class="v-flex">
    <Label label="区块 A" />
    <Separator />
    <Label label="区块 B" />
</div>
```

## 常见错误

期望 `vertical` / `horizontal` RML 属性 — codegen **未映射**方向属性，默认样式由 gpui-component 决定。

## 相关组件

- [label.md](./label.md)

## RML 未覆盖的 API

`.vertical()`、`.horizontal()` 等需 Rust 手写。

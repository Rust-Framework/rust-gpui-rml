# Label

## 概述

`Label` 路由到 `rml_ui::Label`，**Stateless** 组件，用于显示文本标签。

## 基本用法

```html
<Label label="用户名" />
<Label label={user_name} font_semibold="" />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 显示文字 |
| `tooltip` | 字符串 | — | 提示 |
| `disabled` | 布尔 | `{expr}` | 禁用样式 |
| `font_*` | 布尔标志 | — | 字体权重 |
| `small` / `large` 等 | 布尔标志 | — | 尺寸 |

## 事件

| 事件 | 说明 |
|------|------|
| `on-click` | 点击（若需交互） |

## 数据绑定

`label={expr}` 最常用。

## 子节点 / 插槽

可选文本子节点替代 `label`。

## 完整示例

```html
<Label label={t("demo.status")} font_medium="" />
```

## 常见错误

1. **与 HTML `<label>` 混淆** — 小写 `label` 是基础轨 `div()`，无 Label 组件样式。

## 相关组件

- [builtin-html.md](./builtin-html.md)

## RML 未覆盖的 API

`.color()`、`.strikethrough()` 等需 Rust 手写。

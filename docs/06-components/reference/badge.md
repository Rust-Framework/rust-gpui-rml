# Badge

## 概述

`Badge` 路由到 `rml_ui::Badge`，**Stateless** 组件，用于小型状态标记。

## 基本用法

```html
<Badge label="新" />
<Badge label={unread_count} primary="" />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 徽章文字 |
| `primary` / `secondary` / `danger` 等 | 布尔标志 | — | 颜色变体 |
| `small` / `large` | 布尔标志 | — | 尺寸 |
| `disabled` | 布尔 | `{expr}` | 禁用 |

## 事件

| 事件 | 说明 |
|------|------|
| `onclick` | 点击 |

## 数据绑定

`label={count}` 动态显示数字或文字。

## 子节点 / 插槽

可选文本子节点替代 `label`。

## 完整示例

```html
<Badge label={badge_text} danger="" />
```

## 常见错误

无特殊陷阱；注意 Badge 通常不可交互，滥用 `onclick` 不符合语义。

## 相关组件

- [tag.md](./tag.md)

## RML 未覆盖的 API

gpui-component Badge 的 `.dot()`、自定义颜色等需 Rust 手写。

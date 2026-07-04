# Slider

## 概述

`Slider` 路由到 `rml_ui::Slider`，**Stateless** 组件，滑块输入。

## 基本用法

```html
<Slider />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `value` | 数字 | `{volume}` | 当前值 |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `label` | 字符串 | `{expr}` | 标签 |
| `small` / `large` | 布尔标志 | — | 尺寸 |

## 事件

| 事件 | 说明 |
|------|------|
| `on-click` | codegen 支持，但滑块值变化通常需 gpui-component 的 `on_change` — **RML 未映射 `onchange` 给 Slider** |

## 数据绑定

```html
<Slider value={volume} disabled={is_muted} />
```

单向 `value` 绑定；值回写需在 Rust 中扩展或使用其他交互模式。

## 子节点 / 插槽

不支持。

## 完整示例

```html
<Label label="音量" />
<Slider value={volume} />
```

## 常见错误

1. **写 `onchange={handler}`** — codegen 仅 Input/TextInput 支持 `onchange`。
2. **期望双向绑定** — 无 `model` 支持；需自定义命令或 Rust 扩展。

## 相关组件

- [switch.md](./switch.md)

## RML 未覆盖的 API

`.min()`、`.max()`、`.step()`、`.on_change()` 等需 Rust 手写。

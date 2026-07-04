# Tag

## 概述

`Tag` 路由到 `rml_ui::Tag`，**Stateless** 组件，用于可关闭或展示性的标签。

## 基本用法

```html
<Tag label="Rust" />
<Tag label={tag_name} on-click={on_tag_click} />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 标签文字 |
| `primary` / `ghost` 等 | 布尔标志 | — | 变体 |
| `small` / `large` | 布尔标志 | — | 尺寸 |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `selected` | 布尔 | `{expr}` | 选中 |

## 事件

| 事件 | 说明 |
|------|------|
| `on-click` | 点击 |

## 数据绑定

`label={expr}`、`selected={expr}`。

## 子节点 / 插槽

可选文本子节点替代 `label`。

## 完整示例

```html
<div class="tag-row">
    <Tag each={tag in tags} label={tag.name} on-click={on_tag_click} />
</div>
```

## 常见错误

RML 无 `on_close` 事件；关闭逻辑需用 `on-click` 在命令中处理。

## 相关组件

- [badge.md](./badge.md)

## RML 未覆盖的 API

`.on_close()`、`.icon()` 等需 Rust 手写。

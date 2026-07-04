# ButtonGroup

## 概述

`ButtonGroup` 路由到 `rml_ui::ButtonGroup`，**Stateless** 组件。用于将多个按钮组合为一组。

## 基本用法

```html
<ButtonGroup>
    <Button label="左" on-click={on_left} />
    <Button label="右" on-click={on_right} />
</ButtonGroup>
```

## 属性

支持 codegen 通用静态/绑定属性（与其他 Stateless 组件相同）：

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 组合标签（若 API 支持） |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `selected` | 布尔 | `{expr}` | 选中 |
| `small` / `large` 等 | 布尔标志 | — | 尺寸 |
| `font_*` | 布尔标志 | — | 字体权重 |

变体属性（`primary`、`ghost` 等）会映射到 builder，是否生效取决于 gpui-component `ButtonGroup` API。

## 事件

| 事件 | 说明 |
|------|------|
| `on-click` | 组级点击（子 Button 各自处理自己的 `on-click`） |

## 数据绑定

`disabled={expr}`、`label={expr}` 等通用绑定。

## 子节点 / 插槽

子节点作为 `.child()` / `.children()` 传入，通常嵌套多个 `<Button>`。

## 完整示例

```html
<ButtonGroup>
    <Button label="保存" primary="" on-click={on_save} />
    <Button label="取消" ghost="" on-click={on_cancel} />
</ButtonGroup>
```

## 常见错误

1. **期望 RML 级 `variant` 属性** — 应写在子 `Button` 上。
2. **无子节点** — 空组无意义。

## 相关组件

- [button.md](./button.md)

## RML 未覆盖的 API

gpui-component `ButtonGroup` 的方向、间距等 builder 需 Rust 手写。

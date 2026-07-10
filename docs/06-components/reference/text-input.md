# TextInput

## 概述

`TextInput` 是 `Input` 的**别名标签**，路由表指向同一类型 `rml_ui::Input`，Stateful 字段同为 `input_state`。行为、属性、事件与 [input.md](./input.md) 完全一致。

## 基本用法

```html
<TextInput placeholder="备注" onchange={on_remark_change} />
```

## 属性

与 [Input](./input.md#属性) 相同。

## 事件

| 事件 | 回调签名 |
|------|----------|
| `onchange` | `fn(&mut self, state: &InputState, cx: &mut Context<Self>)` |
| `on-click` | `fn(&mut self, ev: &ClickEvent, cx: &mut Context<Self>)` |

## 数据绑定

- 使用 `value={field}` 自动双向绑定，行为与 `<Input>` 一致
- `ref` 模式需声明 `input_state: Entity<InputState>`

## 子节点 / 插槽

同 Input：仅可选文本子节点作 `label`。

## 完整示例

```html
<TextInput placeholder="搜索" onchange={on_search_change} />
```

## 常见错误

与 [input.md](./input.md#常见错误) 相同。

## 相关组件

- [input.md](./input.md)
- [builtin-html.md](./builtin-html.md)

## RML 未覆盖的 API

同 Input。

# Checkbox

## 概述

`Checkbox` 路由到 `rml_ui::Checkbox`，**Stateless** 组件。

## 基本用法

```html
<Checkbox label="记住我" on-click={on_toggle_remember} />
<Checkbox label={agree_label} selected={is_agreed} on-click={on_toggle_agree} />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 标签文字 |
| `tooltip` | 字符串 | — | 提示 |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `selected` / `checked` | 布尔 | `{expr}` | 选中态（`checked` 映射为 `.selected()`） |
| `small` / `large` 等 | 布尔标志 | — | 尺寸 |

## 事件

| 事件 | 回调 |
|------|------|
| `on-click` | `fn(&mut self, ev: &ClickEvent, cx: &mut Context<Self>)` |

## 数据绑定

- `checked={field}` — **自动双向绑定**（Stateless EventClick 机制）
  - 正向：`field` 值 → `.checked(bool)` 显示选中态
  - 反向：点击 → `on_click(&bool)` 事件自动回写 `field` + `bump_version` + `cx.notify()`
  - 无需声明 `on-click` 手动回写，框架自动注入合并回调
- `selected={expr}` — 单向显示选中态（不触发双向绑定）

```html
<!-- 自动双向绑定：点击即回写 agree 字段 -->
<Checkbox label="同意条款" checked={agree} />
```

## 子节点 / 插槽

可选文本子节点替代 `label`。

## 完整示例

```html
<Checkbox label="启用通知" selected={notify_enabled} on-click={on_toggle_notify} />
```

## 常见错误

1. **只绑 `selected` 不写 `on-click`** — `selected` 是单向绑定；双向绑定请用 `checked={field}`。

## 相关组件

- [switch.md](./switch.md)

## RML 未覆盖的 API

`.indeterminate()` 等 gpui-component 方法需 Rust 手写。

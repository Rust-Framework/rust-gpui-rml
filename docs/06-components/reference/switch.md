# Switch

## 概述

`Switch` 路由到 `rml_ui::Switch`，**Stateless** 组件，开关切换。

## 基本用法

```html
<Switch />
<Switch selected={dark_mode} on-click={on_toggle_dark} />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `selected` / `checked` | 布尔 | `{expr}` | 开关状态 |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `label` | 字符串 | `{expr}` | 标签 |
| `small` / `large` | 布尔标志 | — | 尺寸 |

## 事件

| 事件 | 说明 |
|------|------|
| `on-click` | 点击切换；在命令中更新 `selected` 对应字段 |

## 数据绑定

- `checked={field}` — **自动双向绑定**（Stateless EventClick 机制，与 Checkbox 一致）
  - 正向：`field` 值 → `.checked(bool)` 显示开关态
  - 反向：点击 → `on_click(&bool)` 事件自动回写 `field`
  - 无需声明 `on-click` 手动回写
- `selected={expr}` — 单向显示开关态（不触发双向绑定）

```html
<!-- 自动双向绑定：点击即回写 notifications 字段 -->
<Switch checked={notifications} />
```

## 子节点 / 插槽

可选文本子节点作 `label`。

## 完整示例

```html
<div class="setting-row">
    <Label label="深色模式" />
    <Switch selected={is_dark} on-click={on_toggle_theme} />
</div>
```

## 常见错误

1. **只绑 `selected` 无 `on-click`** — `selected` 是单向绑定；双向绑定请用 `checked={field}`。
2. **与 Checkbox 混淆** — Switch 用于二元设置，Checkbox 用于多选/同意场景。

## 相关组件

- [checkbox.md](./checkbox.md)

## RML 未覆盖的 API

`.on_change()`、加载态等需 Rust 手写。

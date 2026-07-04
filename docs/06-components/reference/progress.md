# Progress

## 概述

`Progress` 路由到 `rml_ui::Progress`，**Stateless** 组件，线性进度条。

## 基本用法

```html
<Progress />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `value` | 数字 | `{progress}` | 进度值（映射 `.value(...)`） |
| `label` | 字符串 | `{expr}` | 标签 |
| `disabled` | 布尔 | `{expr}` | 禁用 |

> **注意**：`value` 绑定走通用 `component_bind_setter`，但 gpui-component `Progress` 的进度 API 可能是 `.value(0.5)` 或百分比方法——请以实际编译结果为准。若绑定无效，在 Rust 中手写构造。

## 事件

| 事件 | 说明 |
|------|------|
| `on-click` | 一般不用于进度条 |

## 数据绑定

```html
<Progress value={upload_percent} />
```

## 子节点 / 插槽

不支持子节点。

## 完整示例

```html
<Progress value={loading_progress} />
<Label label={progress_label} />
```

## 常见错误

1. **期望 `max` / `percent` RML 属性** — codegen 未映射，需 Rust 扩展。
2. **value 超出 0–1 或 0–100** — 取决于 gpui-component 约定，查阅其文档。

## 相关组件

- [progress-circle.md](./progress-circle.md)

## RML 未覆盖的 API

`.max()`、条纹动画、状态色等需 Rust 手写。

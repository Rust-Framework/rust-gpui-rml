# ProgressCircle

## 概述

`ProgressCircle` 路由到 `rml_ui::ProgressCircle`，**Stateless** 组件，环形进度指示器。

## 基本用法

```html
<ProgressCircle />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `value` | 数字 | `{progress}` | 进度值 |
| `label` | 字符串 | `{expr}` | 中心或旁路标签 |
| `small` / `large` | 布尔标志 | — | 尺寸 |
| `disabled` | 布尔 | `{expr}` | 禁用 |

## 事件

无常用 RML 事件。

## 数据绑定

```html
<ProgressCircle value={sync_progress} />
```

## 子节点 / 插槽

不支持。

## 完整示例

```html
<div class="loading-pane">
    <ProgressCircle value={0.75} />
    <Label label="加载中…" />
</div>
```

## 常见错误

与 [progress.md](./progress.md) 类似：`value` 语义需与 gpui-component 一致。

## 相关组件

- [progress.md](./progress.md)

## RML 未覆盖的 API

`.indeterminate()`、自定义描边等需 Rust 手写。

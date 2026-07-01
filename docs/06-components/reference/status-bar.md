# status_bar

## 概述

小写标签 `<status_bar>` 路由到 `rml_ui::RmlStatusBar`，包装 gpui-component `StatusBar` 并提供 MVVM 绑定。通过 `items={status_items}` 绑定 `StatusBarItems`（`Vec<Arc<dyn IStatusBarItem>>`）。

> 区别于 PascalCase `<StatusBar>`（gpui-component 原生容器，见 [gpui-status-bar.md](./gpui-status-bar.md)）。Shell 应用应使用 `<status_bar>`。

## 基本用法

```html
<slot_footer>
    <status_bar items={status_items} />
</slot_footer>
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `items` | `StatusBarItems` | `{status_items}` | 状态栏项列表 |

## 事件

无 RML 级事件。

## 数据绑定

### 从贡献点映射

`demo/src/shell/bindings.rs`：

```rust
pub fn status_items_from_host<C>(cx: &gpui::Context<C>, host_id: &str) -> StatusBarItems {
    // 读取 status host → StatusBarItem::new(name).align(Left|Right)
}
```

ViewModel：

```rust
status_items: StatusBarItems,

fn refresh_shell_bindings(&mut self, cx: &mut Context<Self>) {
    self.status_items = bindings::status_items_from_host(cx, hosts::STATUS);
}
```

### `IStatusBarItem` 接口

| 方法 | 说明 |
|------|------|
| `content()` | 显示文字 |
| `align()` | `Left` / `Right` / `Center` |

## 子节点 / 插槽

不支持子节点。

## 完整示例

`demo/src/shell/main_window.rml`：

```html
<slot_footer>
    <status_bar items={status_items} />
</slot_footer>
```

## 常见错误

1. **使用 `<StatusBar items={...}>`** — PascalCase `StatusBar` 无 `items` 绑定，应使用小写 `status_bar`。
2. **期望复杂控件** — 当前仅支持文字内容 + 对齐，不含按钮或进度条。

## 相关组件

- [gpui-status-bar.md](./gpui-status-bar.md) — gpui-component 原生 StatusBar
- [activity-bar.md](./activity-bar.md) — 同类 MVVM Shell 模式

## RML 未覆盖的 API

gpui-component `StatusBar` 的 `.left()` / `.right()` 链式 API 由 `RmlStatusBar` 内部根据 `align` 自动调用；自定义 widget 需 Rust 扩展。

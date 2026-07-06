# ActivityBar

## 概述

`ActivityBar` 是 VS Code 风格的左侧活动栏，由 `rml_ui::ActivityBar` 实现。支持通过 `panels` / `actions` 数据绑定驱动图标栏，子节点渲染到**活动面板内容区**。

典型模式：贡献点 Host → ViewModel 映射 → RML 绑定。见 [贡献点架构](../../09-architecture/contribution-system.md)。

## 基本用法

```html
<ActivityBar panels={activity_panels} />
```

面板内容由各 `IActivityPanel::panel()` 提供（`#[contribute(..., mode = Panel)]` + `render_contribution_visual`）。

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `panels` | `Vec<Arc<dyn IActivityPanel>>` | `{activity_panels}` | 面板图标与激活态 |
| `actions` | `Vec<Arc<dyn IActivityAct>>` | `{actions}` | 底部动作按钮列表（可选） |

静态属性（`label`、`primary` 等通用属性）对 ActivityBar **无效**。

## 事件

| 事件 | 回调签名 | 说明 |
|------|----------|------|
| `on_panel_change` | `fn(&mut self, panel_id: &SharedString, cx: &mut Context<Self>)` | 用户点击面板图标；再次点击已激活面板可折叠（由 ViewModel 处理） |

## 数据绑定

### ViewModel 字段

```rust
activity_panels: Vec<Arc<dyn IActivityPanel>>,
active_panel_id: String,
```

### 从贡献点映射

Demo 使用 `shell_chrome::map_activity_panels`。带 `bindings` 的 `#[contributehost]` 在贡献变更时自动刷新。

### `IActivityPanel` 接口

| 方法 | 说明 |
|------|------|
| `id()` | 面板 ID |
| `icon()` | `Option<IconSpec>` —— 图标规格，详见 [icon.md](./icon.md) |
| `title()` | 工具提示文字 |
| `panel()` | 面板内容（视觉贡献渲染） |

面板内容由 `panel()` 提供，不在 RML 子节点中声明。

## 完整示例

`demo/src/shell/main_window.rml` + `main_window.rml.rs`：

```html
<template slot="left">
    <ActivityBar panels={activity_panels} on_panel_change="on_panel_change">
        <div if={active_panel_id == "samples"} class="nav-tree">
            <Tree on_activate="on_case_activate" />
        </div>
    </ActivityBar>
</template>
```

```rust
#[command]
pub fn on_panel_change(&mut self, id: &SharedString, cx: &mut Context<Self>) {
    let new_id = id.to_string();
    if self.active_panel_id == new_id {
        self.active_panel_id = String::new(); // 折叠
    } else {
        self.active_panel_id = new_id;
    }
    self.refresh_shell_bindings(cx);
}
```

## 常见错误

1. **在 `panels` 里塞 UI 内容** — `panels` 只传元数据；内容放子节点。
2. **忘记 `refresh_shell_bindings`** — 切换 `active_panel_id` 后需重建 `Vec<Arc<dyn IActivityPanel>>` 以更新 `is_activated()`。
3. **子节点始终渲染** — 未用 `if` 过滤时，所有面板内容会叠在一起。

## 相关组件

- [tree.md](./tree.md) — 活动面板内常用案例树
- [status-bar.md](./status-bar.md) — 同类 MVVM Shell 控件

## RML 未覆盖的 API

`.width(px)` 等 builder 方法需在 Rust 中扩展 `ActivityBar` 构造。

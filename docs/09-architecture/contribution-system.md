# 9.7 贡献点架构（Contribution System）

> **本章目标**：理解 RML 贡献点作为**通用扩展基础设施**的定位——框架只提供契约与运行时；UI 由 MVVM 数据绑定驱动，不预设业务 `host_id`。

## 分层：框架 vs 应用

| 层次 | 提供什么 | 不提供什么 |
|------|---------|-----------|
| **rml-core** | `IContribution`、`IContributionHost`、`IContributionRegistry`、`ContributionOptions`、`VisualMode` 元数据 | 任何 `host_id`、Shell 布局、专用呈现组件 |
| **rml-app** | `ContributionRegistry`、`ensure_host`、`build_contribution_tree` | Demo 业务、预置扩展点 |
| **rml-ui** | `ActivityBar`、`status_bar` 等 MVVM 控件 | 贡献点专用渲染器 |
| **应用（Demo）** | 自定 `host_id`、ViewModel 映射、RML 声明式绑定 | — |

## 核心抽象

### `IContributionHost` — 贡献数据管理器（非 UI）

Host 维护某 `host_id` 下的**贡献元数据列表** + 变更通知（`version` / `on_changed`）：

```text
功能模块 register → Host.entries() → ViewModel 映射 → RML 控件绑定
```

**不是 UI 组件。** 所有 UI 呈现均通过 MVVM：

1. Host 变更触发 `on_changed`
2. ViewModel 读取 `entries()`，映射为控件数据类型
3. RML 声明式绑定，数据变化自动刷新 UI

### 最佳实践：活动栏

```mermaid
flowchart LR
    Mod[功能模块 register] --> Host[activity-bar host]
    Host --> VM["ViewModel activity_panels"]
    VM --> RML["ActivityBar panels={...}"]
    Host -->|on_changed| VM
```

- **Host**：维护活动栏项元数据（id、name、icon、`VisualMode::Panel`）
- **ViewModel**：`activity_panels_from_host()` → `ActivityPanels`
- **RML**：`<ActivityBar panels={activity_panels}>`；面板内容由 Shell 在 RML 中按 `active_panel_id` 声明

案例树同理：`case-tree` host（纯数据）→ `tree_items_from_contributions()` → `<Tree>`。

状态栏：`status` host → `status_items_from_host()` → `<status_bar items={status_items}>`。

## 统一注册 API

```rust
registry.register(host_id, Arc::new(contribution), options, cx);  // T: Registerable
```

`#[contribute]` 宏生成 `IContribution` + 数据注册函数。`visual_mode` / `placement` 是**消费方元数据**，不是框架渲染指令。

## `ContributionOptions`

```rust
ContributionOptions::new()
    .order(1)
    .parent_id("parent-id")
    .visual_mode(VisualMode::Panel)   // 供 ViewModel 筛选/映射
    .placement(VisualPlacement::Left)
```

## Demo 参考（非框架契约）

`demo/src/shell/hosts.rs` — 应用自定 host id。

`demo/src/shell/bindings.rs` — Host → `ActivityPanels` / `StatusBarItems` 映射（应用层，非框架）。

`demo/src/shell/main_window.rml`：

```xml
<ActivityBar panels={activity_panels} on_panel_change="on_panel_change">
    <div if={active_panel_id == "samples"} class="nav-tree">
        <Tree on_activate="on_case_activate" />
    </div>
</ActivityBar>
<status_bar items={status_items} />
```

## 参考代码

- 契约：`crates/core/src/contribution.rs`
- 运行时：`crates/app/src/contribution/`
- Demo 范例：`demo/src/shell/bindings.rs`、`demo/src/features/`

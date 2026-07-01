# 9.7 贡献点架构（Contribution System）

> 贡献点是**扩展注册表**，不是 Shell 框架。`rml_app` 只提供 register / entries / subscribe；UI 映射与业务桥接在应用层。

## 框架提供什么

| API | 作用 |
|-----|------|
| `IContributionHost::ID` | 扩展点命名空间 |
| `ContributionExt` | `add` / `register` / `unregister` / `remove` |
| `contribution_entries(host_id, cx)` | 读取已注册条目 |
| `subscribe_host_changes(host_id, cx, f)` | 条目变更通知 |

**不提供**：ActivityBar 映射、案例激活、菜单构建、树形 UI 适配。

## 应用层负责什么（demo 参考）

| 模块 | 职责 |
|------|------|
| `demo/shell/shell_chrome.rs` | slot → Menu / ActivityBar / StatusBar / Tree |
| `demo/shell/case_activation.rs` | 树节点 → MainWindow 开 Tab |
| `MainWindow::refresh_bindings` | 把 registry 同步到 ViewModel 字段 |

## 无 UI 的 Host

```rust
#[contributehost(id = "app.db")]
pub struct DbProviderHost;

// ctor → cx.add("app.db")
// 消费者：
for entry in contribution_entries(DbProviderHost::ID, cx) { ... }
```

## 带 ViewModel 刷新的 Host（仍非框架 UI）

```rust
#[contributehost(id = "demo.shell", bindings = "refresh_bindings")]
#[window]
pub struct MainWindow { ... }
```

宏生成 `__rml_attach_contribution_bindings`：首次 render 时 `subscribe_host_changes` + 调用 `refresh_bindings`。**刷新什么、如何映射到 RML 字段， entirely 应用代码。**

## `#[contribute]`

```rust
#[contribute(host = MainWindow, id = "x", name = "...", slot = "menu")]
```

`slot` 语义由应用定义；demo 约定 `menu` / `activity` / `status` / `case`。

## 变更通知

唯一推荐通道：`subscribe_host_changes`。

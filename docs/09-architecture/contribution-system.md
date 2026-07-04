# 9.7 贡献点架构（Contribution System）

> 贡献点是**扩展注册表**，不是 Shell 框架。`rml_app` 只提供 registry / host / bootstrap；UI 映射与业务桥接在应用层。

## 框架提供什么

| API | 作用 |
|-----|------|
| `IContributionHost::ID` | 扩展点命名空间（`pub const ID: &'static str`）|
| `IContributionHost::add / remove` | Host 直接受理/移除贡献（`&self` + 内部 `RwLock`）|
| `IContributionRegistry::add / remove` | 注册/注销 Host 本身 |
| `IContributionRegistry::register / unregister` | 按 host_id 注册/注销单条贡献 |
| `IAppContextExt::get_contribution_registry()` | 从 `App` / `Context` 获取 registry |
| `bootstrap_host_contributions(cx, host_id)` | 触发 build.rs 生成的 `#[contribute]` 批量注册 |

**不提供**：ActivityBar 映射、案例激活、菜单构建、树形 UI 适配、变更通知订阅。

## 应用层负责什么（demo 参考）

| 模块 | 职责 |
|------|------|
| `demo/shell/main_window.rml.rs` | `on_loaded` 中创建 host handle、注册到 registry、投影到 ViewModel |
| `demo/shell/case_view_model.rs` | `IVisualContribution` → `CaseViewModel` 解包 |
| `demo/shell/menu_view_model.rs` | `ICommand` → `MenuViewModel` 解包 |
| `demo/shell/status_view_model.rs` | `IVisualContribution` → `StatusViewModel` 解包 |

## Host 注册流程

```rust
#[contributehost(id = "app.db")]
pub struct DbProviderHost;

// 用户手写 host handle（实现 IContributionHost）：
pub struct DbProviderHostHandle {
    id: &'static str,
    entries: Arc<RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>>,
}

impl IContributionHost for DbProviderHostHandle {
    fn id(&self) -> &'static str { self.id }
    fn add(&self, c: Arc<dyn IContribution>, opts: Option<ContributionOptions>) {
        self.entries.write().unwrap().push((c, opts.unwrap_or_default()));
    }
    fn remove(&self, cid: &str) {
        self.entries.write().unwrap().retain(|(c, _)| c.id() != cid);
    }
}

// 在 on_loaded 中注册：
let handle = Arc::new(DbProviderHostHandle { id: DbProviderHost::ID, entries: Default::default() });
cx.get_contribution_registry().add(handle.clone());
bootstrap_host_contributions(cx, DbProviderHost::ID); // 触发 #[contribute] 批量注册
```

## 带 ViewModel 投影的 Host（MainWindow）

```rust
#[contributehost(id = "demo.shell")]
#[window]
pub struct MainWindow { ... }
```

`on_loaded` 中：
1. 创建 `MainWindowHostHandle`（实现 `IContributionHost`，持共享 `entries`）
2. `cx.get_contribution_registry().add(Arc::new(handle))`
3. `bootstrap_host_contributions(cx, Self::ID)` 触发 `#[contribute]` 注册
4. 从 `entries` 投影到 `cases / menus / status / activities` ViewModel 集合
5. RML 数据绑定自动驱动 UI

**投影什么、如何映射到 RML 字段， entirely 应用代码。**

## `#[contribute]`

```rust
#[contribute(host = MainWindow, id = "x", name = "...", slot = "menu")]
```

`slot` 语义由应用定义；demo 约定 `menu` / `activity` / `status` / `case`。

## 设计要点

- **Host 直接实现 `IContributionHost`**：无中间 handle 包装层，框架不存储贡献数据
- **无变更通知**：Host 在 `add/remove` 中自行决定是否触发刷新（如 `cx.notify()`）
- **无 `bindings` 参数**：`#[contributehost]` 仅接受 `id`，ViewModel 投影由 `on_loaded` 手写
- **`bootstrap_host_contributions`**：build.rs 生成 `register_rml_contributions_for(cx, host_id)`，按 host_id 路由调用所有 `#[contribute(host_id = "...")]` 的 `__rml_register_*` 函数
